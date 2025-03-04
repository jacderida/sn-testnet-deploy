// Copyright (c) 2023, MaidSafe.
// All rights reserved.
//
// This SAFE Network Software is licensed under the BSD-3-Clause license.
// Please see the LICENSE file for more details.

use crate::{
    ansible::{
        inventory::AnsibleInventoryType,
        provisioning::{PrivateNodeProvisionInventory, ProvisionOptions},
    },
    error::{Error, Result},
    get_bootstrap_cache_url, get_genesis_multiaddr, get_multiaddr, DeploymentInventory,
    DeploymentType, InfraRunOptions, NodeType, TestnetDeployer,
};
use colored::Colorize;
use evmlib::common::U256;
use log::debug;
use std::{collections::HashSet, time::Duration};

#[derive(Clone)]
pub struct UpscaleOptions {
    pub ansible_verbose: bool,
    pub ant_version: Option<String>,
    pub current_inventory: DeploymentInventory,
    pub desired_full_cone_private_node_count: Option<u16>,
    pub desired_full_cone_private_node_vm_count: Option<u16>,
    pub desired_node_count: Option<u16>,
    pub desired_node_vm_count: Option<u16>,
    pub desired_peer_cache_node_count: Option<u16>,
    pub desired_peer_cache_node_vm_count: Option<u16>,
    pub desired_symmetric_private_node_count: Option<u16>,
    pub desired_symmetric_private_node_vm_count: Option<u16>,
    pub desired_uploader_vm_count: Option<u16>,
    pub desired_uploaders_count: Option<u16>,
    pub funding_wallet_secret_key: Option<String>,
    pub gas_amount: Option<U256>,
    pub interval: Duration,
    pub infra_only: bool,
    pub max_archived_log_files: u16,
    pub max_log_files: u16,
    pub plan: bool,
    pub public_rpc: bool,
    pub provision_only: bool,
    pub token_amount: Option<U256>,
}

impl TestnetDeployer {
    pub async fn upscale(&self, options: &UpscaleOptions) -> Result<()> {
        let is_bootstrap_deploy = matches!(
            options
                .current_inventory
                .environment_details
                .deployment_type,
            DeploymentType::Bootstrap
        );

        if is_bootstrap_deploy
            && (options.desired_peer_cache_node_count.is_some()
                || options.desired_peer_cache_node_vm_count.is_some()
                || options.desired_uploader_vm_count.is_some())
        {
            return Err(Error::InvalidUpscaleOptionsForBootstrapDeployment);
        }

        let desired_peer_cache_node_vm_count = options
            .desired_peer_cache_node_vm_count
            .unwrap_or(options.current_inventory.peer_cache_node_vms.len() as u16);
        if desired_peer_cache_node_vm_count
            < options.current_inventory.peer_cache_node_vms.len() as u16
        {
            return Err(Error::InvalidUpscaleDesiredPeerCacheVmCount);
        }
        debug!("Using {desired_peer_cache_node_vm_count} for desired Peer Cache node VM count");

        let desired_node_vm_count = options
            .desired_node_vm_count
            .unwrap_or(options.current_inventory.node_vms.len() as u16);
        if desired_node_vm_count < options.current_inventory.node_vms.len() as u16 {
            return Err(Error::InvalidUpscaleDesiredNodeVmCount);
        }
        debug!("Using {desired_node_vm_count} for desired node VM count");

        let desired_full_cone_private_node_vm_count = options
            .desired_full_cone_private_node_vm_count
            .unwrap_or(options.current_inventory.full_cone_private_node_vms.len() as u16);
        if desired_full_cone_private_node_vm_count
            < options.current_inventory.full_cone_private_node_vms.len() as u16
        {
            return Err(Error::InvalidUpscaleDesiredFullConePrivateNodeVmCount);
        }
        debug!("Using {desired_full_cone_private_node_vm_count} for desired full cone private node VM count");

        let desired_symmetric_private_node_vm_count = options
            .desired_symmetric_private_node_vm_count
            .unwrap_or(options.current_inventory.symmetric_private_node_vms.len() as u16);
        if desired_symmetric_private_node_vm_count
            < options.current_inventory.symmetric_private_node_vms.len() as u16
        {
            return Err(Error::InvalidUpscaleDesiredSymmetricPrivateNodeVmCount);
        }
        debug!("Using {desired_symmetric_private_node_vm_count} for desired full cone private node VM count");

        let desired_uploader_vm_count = options
            .desired_uploader_vm_count
            .unwrap_or(options.current_inventory.uploader_vms.len() as u16);
        if desired_uploader_vm_count < options.current_inventory.uploader_vms.len() as u16 {
            return Err(Error::InvalidUpscaleDesiredUploaderVmCount);
        }
        debug!("Using {desired_uploader_vm_count} for desired uploader VM count");

        let desired_peer_cache_node_count = options
            .desired_peer_cache_node_count
            .unwrap_or(options.current_inventory.peer_cache_node_count() as u16);
        if desired_peer_cache_node_count < options.current_inventory.peer_cache_node_count() as u16
        {
            return Err(Error::InvalidUpscaleDesiredPeerCacheNodeCount);
        }
        debug!("Using {desired_peer_cache_node_count} for desired peer cache node count");

        let desired_node_count = options
            .desired_node_count
            .unwrap_or(options.current_inventory.node_count() as u16);
        if desired_node_count < options.current_inventory.node_count() as u16 {
            return Err(Error::InvalidUpscaleDesiredNodeCount);
        }
        debug!("Using {desired_node_count} for desired node count");

        let desired_full_cone_private_node_count = options
            .desired_full_cone_private_node_count
            .unwrap_or(options.current_inventory.full_cone_private_node_count() as u16);
        if desired_full_cone_private_node_count
            < options.current_inventory.full_cone_private_node_count() as u16
        {
            return Err(Error::InvalidUpscaleDesiredFullConePrivateNodeCount);
        }
        debug!(
            "Using {desired_full_cone_private_node_count} for desired full cone private node count"
        );

        let desired_symmetric_private_node_count = options
            .desired_symmetric_private_node_count
            .unwrap_or(options.current_inventory.symmetric_private_node_count() as u16);
        if desired_symmetric_private_node_count
            < options.current_inventory.symmetric_private_node_count() as u16
        {
            return Err(Error::InvalidUpscaleDesiredSymmetricPrivateNodeCount);
        }
        debug!(
            "Using {desired_symmetric_private_node_count} for desired symmetric private node count"
        );

        let mut infra_run_options = InfraRunOptions::generate_existing(
            &options.current_inventory.name,
            &self.terraform_runner,
            &options.current_inventory.environment_details,
        )
        .await?;
        infra_run_options.peer_cache_node_vm_count = Some(desired_peer_cache_node_vm_count);
        infra_run_options.node_vm_count = Some(desired_node_vm_count);
        infra_run_options.full_cone_private_node_vm_count =
            Some(desired_full_cone_private_node_vm_count);
        infra_run_options.symmetric_private_node_vm_count =
            Some(desired_symmetric_private_node_vm_count);
        infra_run_options.uploader_vm_count = Some(desired_uploader_vm_count);

        if options.plan {
            self.plan(&infra_run_options)?;
            return Ok(());
        }

        self.create_or_update_infra(&infra_run_options)
            .map_err(|err| {
                println!("Failed to create infra {err:?}");
                err
            })?;

        if options.infra_only {
            return Ok(());
        }

        let mut provision_options = ProvisionOptions {
            binary_option: options.current_inventory.binary_option.clone(),
            chunk_size: None,
            client_env_variables: None,
            downloaders_count: 0,
            enable_telegraf: true,
            evm_network: options
                .current_inventory
                .environment_details
                .evm_network
                .clone(),
            evm_data_payments_address: options
                .current_inventory
                .environment_details
                .evm_data_payments_address
                .clone(),
            evm_payment_token_address: options
                .current_inventory
                .environment_details
                .evm_payment_token_address
                .clone(),
            evm_rpc_url: options
                .current_inventory
                .environment_details
                .evm_rpc_url
                .clone(),
            full_cone_private_node_count: desired_full_cone_private_node_count,
            funding_wallet_secret_key: options.funding_wallet_secret_key.clone(),
            interval: options.interval,
            log_format: None,
            logstash_details: None,
            name: options.current_inventory.name.clone(),
            network_id: options.current_inventory.environment_details.network_id,
            node_count: desired_node_count,
            node_env_variables: None,
            max_archived_log_files: options.max_archived_log_files,
            max_log_files: options.max_log_files,
            output_inventory_dir_path: self
                .working_directory_path
                .join("ansible")
                .join("inventory"),
            peer_cache_node_count: desired_peer_cache_node_count,
            public_rpc: options.public_rpc,
            rewards_address: options
                .current_inventory
                .environment_details
                .rewards_address
                .clone(),
            symmetric_private_node_count: desired_symmetric_private_node_count,
            ant_version: options.ant_version.clone(),
            uploaders_count: options.desired_uploaders_count,
            gas_amount: options.gas_amount,
            token_amount: None,
        };
        let mut node_provision_failed = false;

        let (initial_multiaddr, initial_ip_addr) = if is_bootstrap_deploy {
            get_multiaddr(&self.ansible_provisioner.ansible_runner, &self.ssh_client).map_err(
                |err| {
                    println!("Failed to get node multiaddr {err:?}");
                    err
                },
            )?
        } else {
            get_genesis_multiaddr(&self.ansible_provisioner.ansible_runner, &self.ssh_client)
                .map_err(|err| {
                    println!("Failed to get genesis multiaddr {err:?}");
                    err
                })?
        };
        let initial_network_contacts_url = get_bootstrap_cache_url(&initial_ip_addr);
        debug!("Retrieved initial peer {initial_multiaddr} and initial network contacts {initial_network_contacts_url}");

        if !is_bootstrap_deploy {
            self.wait_for_ssh_availability_on_new_machines(
                AnsibleInventoryType::PeerCacheNodes,
                &options.current_inventory,
            )?;
            self.ansible_provisioner
                .print_ansible_run_banner("Provision Peer Cache Nodes");
            match self.ansible_provisioner.provision_nodes(
                &provision_options,
                Some(initial_multiaddr.clone()),
                Some(initial_network_contacts_url.clone()),
                NodeType::PeerCache,
            ) {
                Ok(()) => {
                    println!("Provisioned Peer Cache nodes");
                }
                Err(err) => {
                    log::error!("Failed to provision Peer Cache nodes: {err}");
                    node_provision_failed = true;
                }
            }
        }

        self.wait_for_ssh_availability_on_new_machines(
            AnsibleInventoryType::Nodes,
            &options.current_inventory,
        )?;
        self.ansible_provisioner
            .print_ansible_run_banner("Provision Normal Nodes");
        match self.ansible_provisioner.provision_nodes(
            &provision_options,
            Some(initial_multiaddr.clone()),
            Some(initial_network_contacts_url.clone()),
            NodeType::Generic,
        ) {
            Ok(()) => {
                println!("Provisioned normal nodes");
            }
            Err(err) => {
                log::error!("Failed to provision normal nodes: {err}");
                node_provision_failed = true;
            }
        }

        let private_node_inventory = PrivateNodeProvisionInventory::new(
            &self.ansible_provisioner,
            Some(desired_full_cone_private_node_vm_count),
            Some(desired_symmetric_private_node_vm_count),
        )?;

        if private_node_inventory.should_provision_full_cone_private_nodes() {
            let full_cone_nat_gateway_inventory = self
                .ansible_provisioner
                .ansible_runner
                .get_inventory(AnsibleInventoryType::FullConeNatGateway, true)?;

            let full_cone_nat_gateway_new_vms: Vec<_> = full_cone_nat_gateway_inventory
                .into_iter()
                .filter(|item| {
                    !options
                        .current_inventory
                        .full_cone_nat_gateway_vms
                        .contains(item)
                })
                .collect();

            for vm in full_cone_nat_gateway_new_vms.iter() {
                self.ssh_client.wait_for_ssh_availability(
                    &vm.public_ip_addr,
                    &self.cloud_provider.get_ssh_user(),
                )?;
            }

            let full_cone_nat_gateway_new_vms = if full_cone_nat_gateway_new_vms.is_empty() {
                None
            } else {
                debug!("Full Cone NAT Gateway new VMs: {full_cone_nat_gateway_new_vms:?}");
                Some(full_cone_nat_gateway_new_vms)
            };

            match self.ansible_provisioner.provision_full_cone(
                &provision_options,
                Some(initial_multiaddr.clone()),
                Some(initial_network_contacts_url.clone()),
                private_node_inventory.clone(),
                full_cone_nat_gateway_new_vms,
            ) {
                Ok(()) => {
                    println!("Provisioned Full Cone nodes and Gateway");
                }
                Err(err) => {
                    log::error!("Failed to provision Full Cone nodes and Gateway: {err}");
                    node_provision_failed = true;
                }
            }
        }

        if private_node_inventory.should_provision_symmetric_private_nodes() {
            self.wait_for_ssh_availability_on_new_machines(
                AnsibleInventoryType::SymmetricNatGateway,
                &options.current_inventory,
            )?;
            self.ansible_provisioner
                .print_ansible_run_banner("Provision Symmetric NAT Gateway");
            self.ansible_provisioner
                .provision_symmetric_nat_gateway(&provision_options, &private_node_inventory)
                .map_err(|err| {
                    println!("Failed to provision symmetric NAT gateway {err:?}");
                    err
                })?;

            self.wait_for_ssh_availability_on_new_machines(
                AnsibleInventoryType::SymmetricPrivateNodes,
                &options.current_inventory,
            )?;
            self.ansible_provisioner
                .print_ansible_run_banner("Provision Symmetric Private Nodes");
            match self.ansible_provisioner.provision_symmetric_private_nodes(
                &mut provision_options,
                Some(initial_multiaddr.clone()),
                Some(initial_network_contacts_url.clone()),
                &private_node_inventory,
            ) {
                Ok(()) => {
                    println!("Provisioned symmetric private nodes");
                }
                Err(err) => {
                    log::error!("Failed to provision symmetric private nodes: {err}");
                    node_provision_failed = true;
                }
            }
        }

        let should_provision_uploaders = options.desired_uploaders_count.is_some()
            || options.desired_uploader_vm_count.is_some();
        if should_provision_uploaders {
            self.wait_for_ssh_availability_on_new_machines(
                AnsibleInventoryType::Uploaders,
                &options.current_inventory,
            )?;
            let genesis_network_contacts = get_bootstrap_cache_url(&initial_ip_addr);
            self.ansible_provisioner
                .print_ansible_run_banner("Provision Uploaders");
            self.ansible_provisioner
                .provision_uploaders(
                    &provision_options,
                    Some(initial_multiaddr.clone()),
                    Some(genesis_network_contacts.clone()),
                )
                .await
                .map_err(|err| {
                    println!("Failed to provision uploaders {err:?}");
                    err
                })?;
        }

        if node_provision_failed {
            println!();
            println!("{}", "WARNING!".yellow());
            println!("Some nodes failed to provision without error.");
            println!("This usually means a small number of nodes failed to start on a few VMs.");
            println!("However, most of the time the deployment will still be usable.");
            println!("See the output from Ansible to determine which VMs had failures.");
        }

        Ok(())
    }

    pub async fn upscale_uploaders(&self, options: &UpscaleOptions) -> Result<()> {
        let is_bootstrap_deploy = matches!(
            options
                .current_inventory
                .environment_details
                .deployment_type,
            DeploymentType::Bootstrap
        );

        if is_bootstrap_deploy {
            return Err(Error::InvalidUploaderUpscaleDeploymentType(
                "bootstrap".to_string(),
            ));
        }

        let desired_uploader_vm_count = options
            .desired_uploader_vm_count
            .unwrap_or(options.current_inventory.uploader_vms.len() as u16);
        if desired_uploader_vm_count < options.current_inventory.uploader_vms.len() as u16 {
            return Err(Error::InvalidUpscaleDesiredUploaderVmCount);
        }
        debug!("Using {desired_uploader_vm_count} for desired uploader VM count");

        let mut infra_run_options = InfraRunOptions::generate_existing(
            &options.current_inventory.name,
            &self.terraform_runner,
            &options.current_inventory.environment_details,
        )
        .await?;
        infra_run_options.uploader_vm_count = Some(desired_uploader_vm_count);

        if options.plan {
            self.plan(&infra_run_options)?;
            return Ok(());
        }

        if !options.provision_only {
            self.create_or_update_infra(&infra_run_options)
                .map_err(|err| {
                    println!("Failed to create infra {err:?}");
                    err
                })?;
        }

        if options.infra_only {
            return Ok(());
        }

        let (initial_multiaddr, initial_ip_addr) =
            get_genesis_multiaddr(&self.ansible_provisioner.ansible_runner, &self.ssh_client)
                .map_err(|err| {
                    println!("Failed to get genesis multiaddr {err:?}");
                    err
                })?;
        let initial_network_contacts_url = get_bootstrap_cache_url(&initial_ip_addr);
        debug!("Retrieved initial peer {initial_multiaddr} and initial network contacts {initial_network_contacts_url}");

        let provision_options = ProvisionOptions {
            binary_option: options.current_inventory.binary_option.clone(),
            chunk_size: None,
            client_env_variables: None,
            downloaders_count: 0,
            enable_telegraf: true,
            evm_data_payments_address: options
                .current_inventory
                .environment_details
                .evm_data_payments_address
                .clone(),
            evm_network: options
                .current_inventory
                .environment_details
                .evm_network
                .clone(),
            evm_payment_token_address: options
                .current_inventory
                .environment_details
                .evm_payment_token_address
                .clone(),
            evm_rpc_url: options
                .current_inventory
                .environment_details
                .evm_rpc_url
                .clone(),
            full_cone_private_node_count: 0,
            funding_wallet_secret_key: options.funding_wallet_secret_key.clone(),
            interval: options.interval,
            log_format: None,
            logstash_details: None,
            name: options.current_inventory.name.clone(),
            network_id: options.current_inventory.environment_details.network_id,
            node_count: 0,
            node_env_variables: None,
            max_archived_log_files: options.max_archived_log_files,
            max_log_files: options.max_log_files,
            output_inventory_dir_path: self
                .working_directory_path
                .join("ansible")
                .join("inventory"),
            peer_cache_node_count: 0,
            public_rpc: options.public_rpc,
            rewards_address: options
                .current_inventory
                .environment_details
                .rewards_address
                .clone(),
            symmetric_private_node_count: 0,
            ant_version: options.ant_version.clone(),
            uploaders_count: options.desired_uploaders_count,
            gas_amount: options.gas_amount,
            token_amount: options.token_amount,
        };

        self.wait_for_ssh_availability_on_new_machines(
            AnsibleInventoryType::Uploaders,
            &options.current_inventory,
        )?;
        self.ansible_provisioner
            .print_ansible_run_banner("Provision Uploaders");
        self.ansible_provisioner
            .provision_uploaders(
                &provision_options,
                Some(initial_multiaddr),
                Some(initial_network_contacts_url),
            )
            .await
            .map_err(|err| {
                println!("Failed to provision uploaders {err:?}");
                err
            })?;

        Ok(())
    }

    fn wait_for_ssh_availability_on_new_machines(
        &self,
        inventory_type: AnsibleInventoryType,
        current_inventory: &DeploymentInventory,
    ) -> Result<()> {
        let inventory = self
            .ansible_provisioner
            .ansible_runner
            .get_inventory(inventory_type, true)?;
        let old_set: HashSet<_> = match inventory_type {
            AnsibleInventoryType::PeerCacheNodes => current_inventory
                .peer_cache_node_vms
                .iter()
                .map(|node_vm| &node_vm.vm)
                .cloned()
                .collect(),
            AnsibleInventoryType::Nodes => current_inventory
                .node_vms
                .iter()
                .map(|node_vm| &node_vm.vm)
                .cloned()
                .collect(),
            AnsibleInventoryType::Uploaders => current_inventory
                .uploader_vms
                .iter()
                .map(|uploader_vm| &uploader_vm.vm)
                .cloned()
                .collect(),
            AnsibleInventoryType::FullConeNatGateway => current_inventory
                .full_cone_nat_gateway_vms
                .iter()
                .cloned()
                .collect(),
            AnsibleInventoryType::SymmetricNatGateway => current_inventory
                .symmetric_nat_gateway_vms
                .iter()
                .cloned()
                .collect(),
            AnsibleInventoryType::FullConePrivateNodes => current_inventory
                .full_cone_private_node_vms
                .iter()
                .map(|node_vm| &node_vm.vm)
                .cloned()
                .collect(),
            AnsibleInventoryType::SymmetricPrivateNodes => current_inventory
                .symmetric_private_node_vms
                .iter()
                .map(|node_vm| &node_vm.vm)
                .cloned()
                .collect(),
            it => return Err(Error::UpscaleInventoryTypeNotSupported(it.to_string())),
        };
        let new_vms: Vec<_> = inventory
            .into_iter()
            .filter(|item| !old_set.contains(item))
            .collect();
        for vm in new_vms.iter() {
            self.ssh_client.wait_for_ssh_availability(
                &vm.public_ip_addr,
                &self.cloud_provider.get_ssh_user(),
            )?;
        }
        Ok(())
    }
}
