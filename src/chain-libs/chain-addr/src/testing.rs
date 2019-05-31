use crate::{Address, AddressReadable, Discrimination, Kind, KindType};
use chain_crypto::{Ed25519, KeyPair, PublicKey};
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for Discrimination {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        match u8::arbitrary(g) % 2 {
            0 => Discrimination::Production,
            1 => Discrimination::Test,
            _ => unreachable!(),
        }
    }
}

impl Arbitrary for KindType {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        match u8::arbitrary(g) % 4 {
            0 => KindType::Single,
            1 => KindType::Group,
            2 => KindType::Account,
            3 => KindType::Multisig,
            _ => unreachable!(),
        }
    }
}

impl Arbitrary for AddressReadable {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        AddressReadable::from_address(&Arbitrary::arbitrary(g))
    }
}

fn arbitrary_public_key<G: Gen>(g: &mut G) -> PublicKey<Ed25519> {
    let kp: KeyPair<Ed25519> = Arbitrary::arbitrary(g);
    kp.into_keys().1
}

impl Arbitrary for Address {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let discrimination = Arbitrary::arbitrary(g);
        let kind = match KindType::arbitrary(g) {
            KindType::Single => Kind::Single(arbitrary_public_key(g)),
            KindType::Group => Kind::Group(arbitrary_public_key(g), arbitrary_public_key(g)),
            KindType::Account => Kind::Account(arbitrary_public_key(g)),
            KindType::Multisig => {
                let mut h = [0u8; 32];
                for i in h.iter_mut() {
                    *i = Arbitrary::arbitrary(g)
                }
                Kind::Multisig(h)
            }
        };
        Address(discrimination, kind)
    }
}
