use crate::{
    crypto::key::SigningKey,
    interfaces::{
        Bft, Block0Configuration, BlockchainConfiguration, GenesisPraos, Initial, InitialUTxO,
        NodeConfig, NodeSecret,
    },
    testing::network_builder::{
        Blockchain as BlockchainTemplate, Node as NodeTemplate, NodeAlias, Random, Wallet,
        WalletAlias, WalletTemplate, WalletType,
    },
};
use chain_crypto::Ed25519;
use chain_impl_mockchain::{
    certificate::{PoolPermissions, PoolSignature},
    chaintypes::ConsensusVersion,
    fee::LinearFee,
    rewards::TaxType,
    transaction::{SingleAccountBindingSignature, TxBuilder},
};
use chain_time::DurationSeconds;
use rand_core::{CryptoRng, RngCore};
use std::collections::HashMap;

/// contains all the data to start or interact with a node
#[derive(Debug, Clone)]
pub struct NodeSetting {
    /// for reference purpose only
    pub alias: NodeAlias,

    /// node secret, this will be passed to the node at start
    /// up of the node. It may contains the necessary crypto
    /// for the node to be a blockchain leader (BFT leader or
    /// stake pool)
    pub secret: NodeSecret,

    pub config: NodeConfig,

    pub node_topology: NodeTemplate,
}

impl NodeSetting {
    pub fn new(
        alias: NodeAlias,
        config: NodeConfig,
        secret: NodeSecret,
        template: NodeTemplate,
    ) -> Self {
        Self {
            alias,
            config: config,
            secret: secret,
            node_topology: template,
        }
    }

    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    pub fn secrets(&self) -> &NodeSecret {
        &self.secret
    }
}

#[derive(Debug)]
pub struct Settings {
    pub nodes: HashMap<NodeAlias, NodeSetting>,

    pub wallets: HashMap<WalletAlias, Wallet>,

    pub block0: Block0Configuration,
}

impl Settings {
    pub fn new<RNG>(
        nodes: HashMap<NodeAlias, NodeSetting>,
        blockchain: BlockchainTemplate,
        rng: &mut Random<RNG>,
    ) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        let mut settings = Settings {
            nodes: nodes,
            wallets: HashMap::new(),
            block0: Block0Configuration {
                blockchain_configuration: BlockchainConfiguration::new(
                    chain_addr::Discrimination::Test,
                    ConsensusVersion::Bft,
                    LinearFee::new(1, 2, 3),
                ),
                initial: Vec::new(),
            },
        };

        settings.populate_trusted_peers();
        settings.populate_block0_blockchain_configuration(&blockchain, rng);
        settings.populate_block0_blockchain_initials(blockchain.wallets(), rng);

        settings
    }

    fn populate_block0_blockchain_configuration<RNG>(
        &mut self,
        blockchain: &BlockchainTemplate,
        rng: &mut Random<RNG>,
    ) where
        RNG: RngCore + CryptoRng,
    {
        let mut blockchain_configuration = &mut self.block0.blockchain_configuration;

        // TODO blockchain_configuration.block0_date = ;
        blockchain_configuration.discrimination = chain_addr::Discrimination::Test;
        blockchain_configuration.block0_consensus = *blockchain.consensus();
        blockchain_configuration.consensus_leader_ids = {
            let mut leader_ids = Vec::new();
            for leader_alias in blockchain.leaders() {
                let identifier = if let Some(node) = self.nodes.get_mut(leader_alias) {
                    if let Some(bft) = &node.secret.bft {
                        bft.signing_key.identifier()
                    } else {
                        let signing_key = SigningKey::generate(rng.rng_mut());
                        let identifier = signing_key.identifier();
                        node.secret.bft = Some(Bft { signing_key });
                        identifier
                    }
                } else {
                    SigningKey::<Ed25519>::generate(rng.rng_mut()).identifier()
                };
                leader_ids.push(identifier.into());
            }
            leader_ids
        };
        blockchain_configuration.slots_per_epoch = *blockchain.slots_per_epoch();
        blockchain_configuration.slot_duration = *blockchain.slot_duration();
        // TODO blockchain_configuration.linear_fees = ;
        blockchain_configuration.kes_update_speed = *blockchain.kes_update_speed();
        blockchain_configuration.consensus_genesis_praos_active_slot_coeff =
            *blockchain.consensus_genesis_praos_active_slot_coeff();
    }

    fn populate_block0_blockchain_initials<'a, RNG, I>(
        &'a mut self,
        wallet_templates: I,
        rng: &mut Random<RNG>,
    ) where
        RNG: RngCore + CryptoRng,
        I: Iterator<Item = &'a WalletTemplate>,
    {
        for wallet_template in wallet_templates {
            // TODO: check the wallet does not already exist ?
            let wallet = match wallet_template.wallet_type() {
                WalletType::UTxO => Wallet::generate_utxo(wallet_template.clone(), rng.rng_mut()),
                WalletType::Account => {
                    Wallet::generate_account(wallet_template.clone(), rng.rng_mut())
                }
            };

            let initial_address = wallet.address();

            // TODO add support for sharing fragment with multiple utxos
            let initial_fragment = Initial::Fund(vec![InitialUTxO {
                address: initial_address,
                value: (*wallet_template.value()).into(),
            }]);

            self.wallets
                .insert(wallet_template.alias().clone(), wallet.clone());
            self.block0.initial.push(initial_fragment);

            if let Some(delegation) = wallet_template.delegate() {
                use chain_impl_mockchain::certificate::{
                    PoolId as StakePoolId, PoolOwnersSigned, SignedCertificate,
                };

                // 1. retrieve the public data (we may need to create a stake pool
                //    registration here)
                let stake_pool_id: StakePoolId = if let Some(node) = self.nodes.get_mut(delegation)
                {
                    if let Some(genesis) = &node.secret.genesis {
                        genesis.node_id.clone().into_digest_of()
                    } else {
                        // create and register the stake pool
                        use chain_impl_mockchain::{
                            certificate::PoolRegistration, key::GenesisPraosLeader,
                        };
                        use rand::{distributions::Standard, Rng as _};
                        let serial: u128 = rng.rng_mut().sample(Standard);
                        let kes_signing_key = SigningKey::generate(rng.rng_mut());
                        let vrf_signing_key = SigningKey::generate(rng.rng_mut());
                        let owner = chain_crypto::SecretKey::<chain_crypto::Ed25519>::generate(
                            rng.rng_mut(),
                        );
                        let stake_pool_info = PoolRegistration {
                            serial,
                            permissions: PoolPermissions::new(1),
                            start_validity: DurationSeconds(0).into(),
                            owners: vec![owner.to_public()],
                            operators: vec![].into(),
                            rewards: TaxType::zero(),
                            reward_account: None,
                            keys: GenesisPraosLeader {
                                kes_public_key: kes_signing_key.identifier().into_public_key(),
                                vrf_public_key: vrf_signing_key.identifier().into_public_key(),
                            },
                        };
                        let node_id = stake_pool_info.to_id();
                        node.secret.genesis = Some(GenesisPraos {
                            sig_key: kes_signing_key,
                            vrf_key: vrf_signing_key,
                            node_id: {
                                let bytes: [u8; 32] = node_id.clone().into();
                                bytes.into()
                            },
                        });

                        let txb = TxBuilder::new()
                            .set_payload(&stake_pool_info)
                            .set_ios(&[], &[])
                            .set_witnesses(&[]);
                        let auth_data = txb.get_auth_data();
                        let sig0 = SingleAccountBindingSignature::new(&auth_data, |d| {
                            owner.sign_slice(&d.0)
                        });
                        let owner_signed = PoolOwnersSigned {
                            signatures: vec![(0, sig0)],
                        };

                        let stake_pool_registration_certificate =
                            SignedCertificate::PoolRegistration(
                                stake_pool_info,
                                PoolSignature::Owners(owner_signed),
                            );

                        self.block0
                            .initial
                            .push(Initial::Cert(stake_pool_registration_certificate.into()));

                        node_id
                    }
                } else {
                    // delegating to a node that does not exist in the topology
                    // so generate valid stake pool registration and delegation
                    // to that node.
                    unimplemented!("delegating stake to a stake pool that is not a node is not supported (yet)")
                };

                // 2. create delegation certificate for the wallet stake key
                // and add it to the block0.initial array
                let delegation_certificate = wallet.delegation_cert_for_block0(stake_pool_id);

                self.block0
                    .initial
                    .push(Initial::Cert(delegation_certificate.into()));
            }
        }
    }

    fn populate_trusted_peers(&mut self) {
        let nodes = self.nodes.clone();
        for (_alias, node) in self.nodes.iter_mut() {
            let mut trusted_peers = Vec::new();

            for trusted_peer in node.node_topology.trusted_peers() {
                let trusted_peer = nodes.get(trusted_peer).unwrap();

                trusted_peers.push(trusted_peer.config.p2p.make_trusted_peer_setting());
            }

            node.config.skip_bootstrap = Some(trusted_peers.is_empty());
            node.config.bootstrap_from_trusted_peers = Some(!trusted_peers.is_empty());
            node.config.p2p.trusted_peers = trusted_peers;
        }
    }
}
