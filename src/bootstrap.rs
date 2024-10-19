// Copyright (c) 2023, MaidSafe.
// All rights reserved.
//
// This SAFE Network Software is licensed under the BSD-3-Clause license.
// Please see the LICENSE file for more details.

use std::path::PathBuf;

use crate::{
    ansible::{inventory::AnsibleInventoryType, provisioning::ProvisionOptions},
    error::Result,
    write_environment_details, BinaryOption, DeploymentType, EnvironmentDetails, EnvironmentType,
    EvmNetwork, InfraRunOptions, LogFormat, NodeType, TestnetDeployer,
};
use colored::Colorize;

#[derive(Clone)]
pub struct BootstrapOptions {
    pub binary_option: BinaryOption,
    pub bootstrap_peer: String,
    pub environment_type: EnvironmentType,
    pub env_variables: Option<Vec<(String, String)>>,
    pub evm_network: EvmNetwork,
    pub log_format: Option<LogFormat>,
    pub max_archived_log_files: u16,
    pub max_log_files: u16,
    pub name: String,
    pub node_count: u16,
    pub node_vm_count: Option<u16>,
    pub output_inventory_dir_path: PathBuf,
    pub private_node_count: u16,
    pub private_node_vm_count: Option<u16>,
    pub rewards_address: String,
}

impl TestnetDeployer {
    pub async fn bootstrap(&self, options: &BootstrapOptions) -> Result<()> {
        let build_custom_binaries = {
            match &options.binary_option {
                BinaryOption::BuildFromSource { .. } => true,
                BinaryOption::Versioned { .. } => false,
            }
        };

        // All the environment types set private_node_vm count to >0 if not specified.
        let should_provision_private_nodes = options
            .private_node_vm_count
            .map(|count| count > 0)
            .unwrap_or(true);

        write_environment_details(
            &self.s3_repository,
            &options.name,
            &EnvironmentDetails {
                deployment_type: DeploymentType::Bootstrap,
                environment_type: options.environment_type.clone(),
                evm_network: options.evm_network.clone(),
                evm_testnet_data: None,
                rewards_address: options.rewards_address.clone(),
            },
        )
        .await?;

        self.create_or_update_infra(&InfraRunOptions {
            bootstrap_node_vm_count: None,
            bootstrap_node_vm_size: None,
            enable_build_vm: build_custom_binaries,
            evm_node_count: None,
            evm_node_vm_size: None,
            genesis_vm_count: None,
            name: options.name.clone(),
            node_vm_count: options.node_vm_count,
            node_vm_size: None,
            private_node_vm_count: options.private_node_vm_count,
            tfvars_filename: options.environment_type.get_tfvars_filename().to_string(),
            uploader_vm_count: None,
            uploader_vm_size: None,
        })
        .await
        .map_err(|err| {
            println!("Failed to create infra {err:?}");
            err
        })?;

        let mut n = 1;
        let mut total = if build_custom_binaries { 2 } else { 1 };
        if should_provision_private_nodes {
            total += 2;
        }

        let mut provision_options = ProvisionOptions::from(options.clone());
        if build_custom_binaries {
            self.ansible_provisioner
                .print_ansible_run_banner(n, total, "Build Custom Binaries");
            self.ansible_provisioner
                .build_safe_network_binaries(&provision_options)
                .await
                .map_err(|err| {
                    println!("Failed to build safe network binaries {err:?}");
                    err
                })?;
            n += 1;
        }

        let mut failed_to_provision = false;

        self.ansible_provisioner
            .print_ansible_run_banner(n, total, "Provision Normal Nodes");
        match self
            .ansible_provisioner
            .provision_nodes(
                &provision_options,
                &options.bootstrap_peer,
                NodeType::Normal,
                None,
            )
            .await
        {
            Ok(()) => {
                println!("Provisioned normal nodes");
            }
            Err(e) => {
                println!("Failed to provision normal nodes: {e:?}");
                failed_to_provision = true;
            }
        }

        if should_provision_private_nodes {
            let private_nodes = self
                .ansible_provisioner
                .ansible_runner
                .get_inventory(AnsibleInventoryType::PrivateNodes, true)
                .await
                .map_err(|err| {
                    println!("Failed to obtain the inventory of private node: {err:?}");
                    err
                })?;

            provision_options.private_node_vms = private_nodes;
            self.ansible_provisioner
                .print_ansible_run_banner(n, total, "Provision NAT Gateway");
            self.ansible_provisioner
                .provision_nat_gateway(&provision_options)
                .await
                .map_err(|err| {
                    println!("Failed to provision NAT gateway {err:?}");
                    err
                })?;

            n += 1;

            self.ansible_provisioner
                .print_ansible_run_banner(n, total, "Provision Private Nodes");
            match self
                .ansible_provisioner
                .provision_private_nodes(&mut provision_options, &options.bootstrap_peer, None)
                .await
            {
                Ok(()) => {
                    println!("Provisioned private nodes");
                }
                Err(err) => {
                    log::error!("Failed to provision private nodes: {err}");
                    failed_to_provision = true;
                }
            }
        }

        if failed_to_provision {
            println!("{}", "WARNING!".yellow());
            println!("Some nodes failed to provision without error.");
            println!("This usually means a small number of nodes failed to start on a few VMs.");
            println!("However, most of the time the deployment will still be usable.");
            println!("See the output from Ansible to determine which VMs had failures.");
        }

        Ok(())
    }
}
