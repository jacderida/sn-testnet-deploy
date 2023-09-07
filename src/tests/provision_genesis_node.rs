// Copyright (c) 2023, MaidSafe.
// All rights reserved.
//
// This SAFE Network Software is licensed under the BSD-3-Clause license.
// Please see the LICENSE file for more details.

use super::super::logstash::LOGSTASH_PORT;
use super::super::{CloudProvider, TestnetDeploy};
use super::setup::*;
use crate::ansible::MockAnsibleRunnerInterface;
use crate::rpc_client::MockRpcClientInterface;
use crate::ssh::MockSshClientInterface;
use color_eyre::Result;
use mockall::predicate::*;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

const CUSTOM_BIN_URL: &str = "https://sn-node.s3.eu-west-2.amazonaws.com/maidsafe/custom_branch/safenode-beta-x86_64-unknown-linux-musl.tar.gz";

#[tokio::test]
async fn should_run_ansible_against_genesis() -> Result<()> {
    // Unfortunately there is just no good way to format these strings so that they are both
    // readable and comparable to the string that gets generated by the provisioning process.
    let extra_vars_doc = r#"{ "provider": "digital-ocean", "testnet_name": "beta", "logstash_stack_name": "main", "logstash_hosts": ["10.0.0.1:5044", "10.0.0.2:5044"] }"#;
    let (tmp_dir, working_dir) = setup_working_directory()?;
    let s3_repository = setup_deploy_s3_repository("beta", &working_dir)?;
    let mut ansible_runner = MockAnsibleRunnerInterface::new();
    ansible_runner
        .expect_inventory_list()
        .times(1)
        .with(eq(
            PathBuf::from("inventory").join(".beta_genesis_inventory_digital_ocean.yml")
        ))
        .returning(|_| Ok(vec![("beta-genesis".to_string(), "10.0.0.10".to_string())]));
    ansible_runner
        .expect_run_playbook()
        .times(1)
        .with(
            eq(PathBuf::from("genesis_node.yml")),
            eq(PathBuf::from("inventory").join(".beta_genesis_inventory_digital_ocean.yml")),
            eq("root".to_string()),
            eq(Some(extra_vars_doc.to_string())),
        )
        .returning(|_, _, _, _| Ok(()));

    let mut ssh_client = MockSshClientInterface::new();
    ssh_client
        .expect_wait_for_ssh_availability()
        .times(1)
        .with(eq("10.0.0.10"), eq("root"))
        .returning(|_, _| Ok(()));

    let testnet = TestnetDeploy::new(
        Box::new(setup_default_terraform_runner("beta")),
        Box::new(ansible_runner),
        Box::new(MockRpcClientInterface::new()),
        Box::new(ssh_client),
        working_dir.to_path_buf(),
        CloudProvider::DigitalOcean,
        Box::new(s3_repository),
    );

    testnet.init("beta").await?;
    testnet
        .provision_genesis_node(
            "beta",
            (
                "main",
                &[
                    SocketAddr::new(IpAddr::V4("10.0.0.1".parse()?), LOGSTASH_PORT),
                    SocketAddr::new(IpAddr::V4("10.0.0.2".parse()?), LOGSTASH_PORT),
                ],
            ),
            None,
        )
        .await?;

    drop(tmp_dir);
    Ok(())
}

#[tokio::test]
async fn should_run_ansible_against_genesis_with_a_custom_binary() -> Result<()> {
    let extra_vars_doc = r#"{ "provider": "digital-ocean", "testnet_name": "beta", "node_archive_url": "CUSTOM_BIN_URL", "logstash_stack_name": "main", "logstash_hosts": ["10.0.0.1:5044", "10.0.0.2:5044"] }"#;
    let (tmp_dir, working_dir) = setup_working_directory()?;
    let s3_repository = setup_deploy_s3_repository("beta", &working_dir)?;
    let mut ansible_runner = MockAnsibleRunnerInterface::new();
    ansible_runner
        .expect_inventory_list()
        .times(1)
        .with(eq(
            PathBuf::from("inventory").join(".beta_genesis_inventory_digital_ocean.yml")
        ))
        .returning(|_| Ok(vec![("beta-genesis".to_string(), "10.0.0.10".to_string())]));
    ansible_runner
        .expect_run_playbook()
        .times(1)
        .with(
            eq(PathBuf::from("genesis_node.yml")),
            eq(PathBuf::from("inventory").join(".beta_genesis_inventory_digital_ocean.yml")),
            eq("root".to_string()),
            eq(Some(
                extra_vars_doc
                    .replace("CUSTOM_BIN_URL", CUSTOM_BIN_URL)
                    .to_string(),
            )),
        )
        .returning(|_, _, _, _| Ok(()));

    let mut ssh_client = MockSshClientInterface::new();
    ssh_client
        .expect_wait_for_ssh_availability()
        .times(1)
        .with(eq("10.0.0.10"), eq("root"))
        .returning(|_, _| Ok(()));

    let testnet = TestnetDeploy::new(
        Box::new(setup_default_terraform_runner("beta")),
        Box::new(ansible_runner),
        Box::new(MockRpcClientInterface::new()),
        Box::new(ssh_client),
        working_dir.to_path_buf(),
        CloudProvider::DigitalOcean,
        Box::new(s3_repository),
    );

    testnet.init("beta").await?;
    testnet
        .provision_genesis_node(
            "beta",
            (
                "main",
                &[
                    SocketAddr::new(IpAddr::V4("10.0.0.1".parse()?), LOGSTASH_PORT),
                    SocketAddr::new(IpAddr::V4("10.0.0.2".parse()?), LOGSTASH_PORT),
                ],
            ),
            Some(("maidsafe".to_string(), "custom_branch".to_string())),
        )
        .await?;

    drop(tmp_dir);
    Ok(())
}
