use crate::{legacy::LegacyNodeController, test::Result};
use crate::{node::NodeController, scenario::Controller};
use jormungandr_testing_utils::{
    testing::{FragmentNode, SyncNode},
    wallet::Wallet,
};
use structopt::{clap::AppSettings, StructOpt};
use crate::VitStationController;
use crate::wallet::WalletProxyController;
use jormungandr_lib::interfaces::Value;

mod describe;
mod send;
mod show;
mod spawn;

pub struct UserInteractionController<'a> {
    controller: &'a mut Controller,
    wallets: Vec<Wallet>,
    nodes: Vec<NodeController>,
    legacy_nodes: Vec<LegacyNodeController>,
    vit_stations: Vec<VitStationController>,
    proxies: Vec<WalletProxyController>
}

impl<'a> UserInteractionController<'a> {
    pub fn new(controller: &'a mut Controller) -> Self {
        let wallets = controller.get_all_wallets();
        Self {
            controller,
            wallets,
            nodes: Vec::new(),
            legacy_nodes: Vec::new(),
            vit_stations: Vec::new(),
            proxies: Vec::new()
        }
    }

    pub fn wallets(&self) -> &[Wallet] {
        &self.wallets
    }

    pub fn wallets_mut(&mut self) -> &mut Vec<Wallet> {
        &mut self.wallets
    }

    pub fn nodes(&self) -> &[NodeController] {
        &self.nodes
    }

    pub fn legacy_nodes(&self) -> &[LegacyNodeController] {
        &self.legacy_nodes
    }

    pub fn legacy_nodes_mut(&mut self) -> &mut Vec<LegacyNodeController> {
        &mut self.legacy_nodes
    }
    pub fn nodes_mut(&mut self) -> &mut Vec<NodeController> {
        &mut self.nodes
    }

    pub fn vit_stations_mut(&mut self) -> &mut Vec<VitStationController> {
        &mut self.vit_stations
    }

    pub fn proxies(&self) -> &[WalletProxyController] {
        &self.proxies
    }

    pub fn vit_stations(&self) -> &[VitStationController] {
        &self.vit_stations
    }

    pub fn proxies_mut(&mut self) -> &mut Vec<WalletProxyController> {
        &mut self.proxies
    }

    pub fn controller(&self) -> &Controller {
        &self.controller
    }

    pub fn controller_mut(&mut self) -> &mut Controller {
        &mut self.controller
    }

    pub fn tally_vote<A: FragmentNode + SyncNode + Sized + Sync + Send>(
        &mut self,
        committee_alias: &str,
        vote_plan_alias: &str,
        via: &A
    ) -> Result<jormungandr_testing_utils::testing::MemPoolCheck> {
        let committee_address = self.controller.wallet(&committee_alias)?.address();
        let vote_plan_def = self.controller.vote_plan(vote_plan_alias)?;

        let mut temp_wallets = self.wallets_mut().clone();
        let committee = temp_wallets
            .iter_mut()
            .find(|x| x.address() == committee_address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", committee_alias));

        let check = self
            .controller
            .fragment_sender()
            .send_public_vote_tally(
                committee,
                &vote_plan_def.into(),
                via,
            )?;

        *self.wallets_mut() = temp_wallets;
        Ok(check)
    }

    pub fn send_transaction<A: FragmentNode + SyncNode + Sized + Sync + Send>(
        &mut self,
        from_str: &str,
        to_str: &str,
        via: &A,
        value: Value,
    ) -> Result<jormungandr_testing_utils::testing::MemPoolCheck> {
        let from_address = self.controller.wallet(&from_str)?.address();
        let to_address = self.controller.wallet(&to_str)?.address();

        let to = self
            .wallets()
            .iter()
            .cloned()
            .find(|x| x.address() == to_address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", to_str));

        let mut temp_wallets = self.wallets_mut().clone();
        let from = temp_wallets
            .iter_mut()
            .find(|x| x.address() == from_address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", from_str));

        let check = self
            .controller
            .fragment_sender()
            .send_transaction(from, &to, via, value)?;
        *self.wallets_mut() = temp_wallets;
        Ok(check)
    }
}

#[derive(StructOpt, Debug)]
#[structopt(setting = AppSettings::NoBinaryName)]
pub enum InteractiveCommand {
    /// Prints nodes related data, like stats,fragments etc.
    Show(show::Show),
    /// Spawn leader or passive node (also legacy)
    Spawn(spawn::Spawn),
    /// Exit interactive mode
    Exit,
    /// Prints wallets, nodes which can be used. Draw topology
    Describe(describe::Describe),
    /// send fragments
    Send(send::Send),
}

fn do_for_all_alias<F: Fn(&NodeController), G: Fn(&LegacyNodeController)>(
    alias: &Option<String>,
    nodes: &[NodeController],
    legacy_nodes: &[LegacyNodeController],
    f: F,
    g: G,
) -> Result<()> {
    if let Some(alias) = alias {
        if let Some(node) = nodes.iter().find(|x| *x.alias() == *alias) {
            f(node);
        }
        if let Some(node) = legacy_nodes.iter().find(|x| *x.alias() == *alias) {
            g(node)
        }
        return Ok(());
    }

    for node in nodes.iter() {
        f(node);
    }
    for node in legacy_nodes.iter() {
        g(node);
    }
    Ok(())
}
