use super::{
    extra_vars::ExtraVarsDocBuilder, AnsibleInventoryType, AnsiblePlaybook, AnsibleRunner,
};
use crate::{
    deploy::DeployOptions,
    error::{Error, Result},
    print_duration, BinaryOption, CloudProvider, LogFormat, SshClient, UpgradeOptions,
};
use semver::Version;
use std::{
    net::{IpAddr, SocketAddr},
    time::Instant,
};

const DEFAULT_BETA_ENCRYPTION_KEY: &str =
    "49113d2083f57a976076adbe85decb75115820de1e6e74b47e0429338cef124a";

pub enum NodeType {
    Bootstrap,
    Normal,
}

pub struct ProvisionOptions {
    pub beta_encryption_key: Option<String>,
    pub binary_option: BinaryOption,
    pub bootstrap_node_count: u16,
    pub env_variables: Option<Vec<(String, String)>>,
    pub log_format: Option<LogFormat>,
    pub logstash_details: Option<(String, Vec<SocketAddr>)>,
    pub name: String,
    pub node_count: u16,
    pub public_rpc: bool,
}

impl From<DeployOptions> for ProvisionOptions {
    fn from(deploy_options: DeployOptions) -> Self {
        ProvisionOptions {
            beta_encryption_key: deploy_options.beta_encryption_key,
            binary_option: deploy_options.binary_option,
            bootstrap_node_count: deploy_options.bootstrap_node_count,
            env_variables: deploy_options.env_variables,
            log_format: deploy_options.log_format,
            logstash_details: deploy_options.logstash_details,
            name: deploy_options.name,
            node_count: deploy_options.node_count,
            public_rpc: deploy_options.public_rpc,
        }
    }
}

#[derive(Clone)]
pub struct AnsibleProvisioner {
    pub ansible_runner: AnsibleRunner,
    pub cloud_provider: CloudProvider,
    pub ssh_client: SshClient,
}

impl AnsibleProvisioner {
    pub fn new(
        ansible_runner: AnsibleRunner,
        cloud_provider: CloudProvider,
        ssh_client: SshClient,
    ) -> Self {
        Self {
            ansible_runner,
            cloud_provider,
            ssh_client,
        }
    }

    pub async fn build_safe_network_binaries(&self, options: &ProvisionOptions) -> Result<()> {
        let start = Instant::now();
        println!("Obtaining IP address for build VM...");
        let build_inventory = self
            .ansible_runner
            .get_inventory(AnsibleInventoryType::Build, true)
            .await?;
        let build_ip = build_inventory[0].1;
        self.ssh_client
            .wait_for_ssh_availability(&build_ip, &self.cloud_provider.get_ssh_user())?;

        println!("Running ansible against build VM...");
        let extra_vars = self.build_binaries_extra_vars_doc(options)?;
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::Build,
            AnsibleInventoryType::Build,
            Some(extra_vars),
        )?;
        print_duration(start.elapsed());
        Ok(())
    }

    pub async fn copy_logs(&self, name: &str, resources_only: bool) -> Result<()> {
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::Logs,
            AnsibleInventoryType::Genesis,
            Some(format!(
                "{{ \"env_name\": \"{name}\", \"resources_only\" : \"{resources_only}\" }}"
            )),
        )?;
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::Logs,
            AnsibleInventoryType::Nodes,
            Some(format!(
                "{{ \"env_name\": \"{name}\", \"resources_only\" : \"{resources_only}\" }}"
            )),
        )?;
        Ok(())
    }

    pub async fn get_all_node_inventory(&self) -> Result<Vec<(String, IpAddr)>> {
        let mut all_node_inventory = self
            .ansible_runner
            .get_inventory(AnsibleInventoryType::Genesis, false)
            .await?;
        all_node_inventory.extend(
            self.ansible_runner
                .get_inventory(AnsibleInventoryType::Nodes, false)
                .await?,
        );
        Ok(all_node_inventory)
    }

    pub async fn provision_genesis_node(&self, options: &ProvisionOptions) -> Result<()> {
        let start = Instant::now();
        let genesis_inventory = self
            .ansible_runner
            .get_inventory(AnsibleInventoryType::Genesis, true)
            .await?;
        let genesis_ip = genesis_inventory[0].1;
        self.ssh_client
            .wait_for_ssh_availability(&genesis_ip, &self.cloud_provider.get_ssh_user())?;
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::Genesis,
            AnsibleInventoryType::Genesis,
            Some(self.build_node_extra_vars_doc(options, NodeType::Bootstrap, None, 1)?),
        )?;
        print_duration(start.elapsed());
        Ok(())
    }

    pub async fn provision_nodes(
        &self,
        options: &ProvisionOptions,
        initial_contact_peer: &str,
        node_type: NodeType,
    ) -> Result<()> {
        let start = Instant::now();
        let (inventory_type, node_count) = match node_type {
            NodeType::Bootstrap => (
                AnsibleInventoryType::BootstrapNodes,
                options.bootstrap_node_count,
            ),
            NodeType::Normal => (AnsibleInventoryType::Nodes, options.node_count),
        };

        self.ansible_runner.run_playbook(
            AnsiblePlaybook::Nodes,
            inventory_type,
            Some(self.build_node_extra_vars_doc(
                options,
                node_type,
                Some(initial_contact_peer.to_string()),
                node_count,
            )?),
        )?;
        print_duration(start.elapsed());
        Ok(())
    }

    pub async fn provision_faucet(
        &self,
        options: &ProvisionOptions,
        genesis_multiaddr: &str,
    ) -> Result<()> {
        let start = Instant::now();
        println!("Running ansible against genesis node to deploy faucet...");
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::Faucet,
            AnsibleInventoryType::Genesis,
            Some(self.build_faucet_extra_vars_doc(options, genesis_multiaddr)?),
        )?;
        print_duration(start.elapsed());
        Ok(())
    }

    pub async fn provision_safenode_rpc_client(
        &self,
        options: &ProvisionOptions,
        genesis_multiaddr: &str,
    ) -> Result<()> {
        let start = Instant::now();
        println!("Running ansible against genesis node to start safenode_rpc_client service...");
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::RpcClient,
            AnsibleInventoryType::Genesis,
            Some(self.build_safenode_rpc_client_extra_vars_doc(options, genesis_multiaddr)?),
        )?;
        print_duration(start.elapsed());
        Ok(())
    }

    pub async fn provision_sn_auditor(
        &self,
        options: &ProvisionOptions,
        genesis_multiaddr: &str,
    ) -> Result<()> {
        let start = Instant::now();
        println!("Running ansible against auditor machine to start sn_auditor service...");
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::Auditor,
            AnsibleInventoryType::Auditor,
            Some(self.build_sn_auditor_extra_vars_doc(options, genesis_multiaddr)?),
        )?;
        print_duration(start.elapsed());
        Ok(())
    }

    pub async fn start_nodes(&self) -> Result<()> {
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::StartNodes,
            AnsibleInventoryType::Genesis,
            None,
        )?;
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::StartNodes,
            AnsibleInventoryType::Nodes,
            None,
        )?;
        Ok(())
    }

    pub async fn upgrade_nodes(&self, options: &UpgradeOptions) -> Result<()> {
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::UpgradeNodes,
            AnsibleInventoryType::Nodes,
            Some(options.get_ansible_vars()),
        )?;
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::UpgradeNodes,
            AnsibleInventoryType::Genesis,
            Some(options.get_ansible_vars()),
        )?;
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::UpgradeFaucet,
            AnsibleInventoryType::Genesis,
            Some(options.get_ansible_vars()),
        )?;
        Ok(())
    }

    pub async fn upgrade_node_manager(&self, version: &Version) -> Result<()> {
        let mut extra_vars = ExtraVarsDocBuilder::default();
        extra_vars.add_variable("version", &version.to_string());
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::UpgradeNodeManager,
            AnsibleInventoryType::Genesis,
            Some(extra_vars.build()),
        )?;
        self.ansible_runner.run_playbook(
            AnsiblePlaybook::UpgradeNodeManager,
            AnsibleInventoryType::Nodes,
            Some(extra_vars.build()),
        )?;
        Ok(())
    }

    pub fn print_ansible_run_banner(&self, n: usize, total: usize, s: &str) {
        let ansible_run_msg = format!("Ansible Run {} of {}: ", n, total);
        let line = "=".repeat(s.len() + ansible_run_msg.len());
        println!("{}\n{}{}\n{}", line, ansible_run_msg, s, line);
    }

    fn build_node_extra_vars_doc(
        &self,
        options: &ProvisionOptions,
        node_type: NodeType,
        bootstrap_multiaddr: Option<String>,
        node_instance_count: u16,
    ) -> Result<String> {
        let mut extra_vars = ExtraVarsDocBuilder::default();
        extra_vars.add_variable("provider", &self.cloud_provider.to_string());
        extra_vars.add_variable("testnet_name", &options.name);

        match node_type {
            NodeType::Bootstrap => {
                extra_vars.add_variable("node_type", "bootstrap_node");
            }
            NodeType::Normal => {
                extra_vars.add_variable("node_type", "generic_node");
            }
        }

        if bootstrap_multiaddr.is_some() {
            extra_vars.add_variable(
                "genesis_multiaddr",
                &bootstrap_multiaddr.ok_or_else(|| Error::GenesisMultiAddrNotSupplied)?,
            );
        }

        extra_vars.add_variable("node_instance_count", &node_instance_count.to_string());
        if let Some(log_format) = options.log_format {
            extra_vars.add_variable("log_format", log_format.as_str());
        }
        if options.public_rpc {
            extra_vars.add_variable("public_rpc", "true");
        }

        extra_vars.add_node_url_or_version(&options.name, &options.binary_option);
        extra_vars.add_node_manager_url(&options.name, &options.binary_option);
        extra_vars.add_node_manager_daemon_url(&options.name, &options.binary_option);

        if let Some(env_vars) = &options.env_variables {
            extra_vars.add_env_variable_list("env_variables", env_vars.clone());
        }

        if let Some((logstash_stack_name, logstash_hosts)) = &options.logstash_details {
            extra_vars.add_variable("logstash_stack_name", logstash_stack_name);
            extra_vars.add_list_variable(
                "logstash_hosts",
                logstash_hosts
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            );
        }

        Ok(extra_vars.build())
    }

    fn build_faucet_extra_vars_doc(
        &self,
        options: &ProvisionOptions,
        genesis_multiaddr: &str,
    ) -> Result<String> {
        let mut extra_vars = ExtraVarsDocBuilder::default();
        extra_vars.add_variable("provider", &self.cloud_provider.to_string());
        extra_vars.add_variable("testnet_name", &options.name);
        extra_vars.add_variable("genesis_multiaddr", genesis_multiaddr);
        extra_vars.add_node_manager_url(&options.name, &options.binary_option);
        extra_vars.add_faucet_url_or_version(&options.name, &options.binary_option);
        Ok(extra_vars.build())
    }

    fn build_safenode_rpc_client_extra_vars_doc(
        &self,
        options: &ProvisionOptions,
        genesis_multiaddr: &str,
    ) -> Result<String> {
        let mut extra_vars = ExtraVarsDocBuilder::default();
        extra_vars.add_variable("provider", &self.cloud_provider.to_string());
        extra_vars.add_variable("testnet_name", &options.name);
        extra_vars.add_variable("genesis_multiaddr", genesis_multiaddr);
        extra_vars.add_rpc_client_url_or_version(&options.name, &options.binary_option);
        Ok(extra_vars.build())
    }

    fn build_sn_auditor_extra_vars_doc(
        &self,
        options: &ProvisionOptions,
        genesis_multiaddr: &str,
    ) -> Result<String> {
        let mut extra_vars: ExtraVarsDocBuilder = ExtraVarsDocBuilder::default();
        extra_vars.add_variable("provider", &self.cloud_provider.to_string());
        extra_vars.add_variable("testnet_name", &options.name);
        extra_vars.add_variable("genesis_multiaddr", genesis_multiaddr);
        extra_vars.add_variable(
            "beta_encryption_key",
            options
                .beta_encryption_key
                .as_ref()
                .unwrap_or(&DEFAULT_BETA_ENCRYPTION_KEY.to_string()),
        );
        extra_vars.add_node_manager_url(&options.name, &options.binary_option);
        extra_vars.add_sn_auditor_url_or_version(&options.name, &options.binary_option);
        Ok(extra_vars.build())
    }

    fn build_binaries_extra_vars_doc(&self, options: &ProvisionOptions) -> Result<String> {
        let mut extra_vars = ExtraVarsDocBuilder::default();
        extra_vars.add_build_variables(&options.name, &options.binary_option);
        Ok(extra_vars.build())
    }
}
