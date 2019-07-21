use super::Error;
use crate::certificate;
use crate::transaction::*;
use crate::value::Value;
use chain_addr::Address;

macro_rules! if_cond_fail_with(
    ($cond: expr, $err: expr) => {
        if $cond {
            Err($err)
        } else {
            Ok(())
        }
    };
);

type LedgerCheck = Result<(), Error>;

/// Check that the output value is valid
pub(super) fn valid_output_value(output: &Output<Address>) -> LedgerCheck {
    if_cond_fail_with!(
        output.value == Value::zero(),
        Error::ZeroOutput {
            output: output.clone()
        }
    )
}

/// check that the transaction input/outputs/witnesses is valid for stake_owner_delegation
pub(super) fn valid_stake_owner_delegation_transaction(
    auth_cert: &AuthenticatedTransaction<Address, certificate::OwnerStakeDelegation>,
) -> LedgerCheck {
    if_cond_fail_with!(
        auth_cert.transaction.inputs.len() != 1
            || auth_cert.witnesses.len() != 1
            || auth_cert.transaction.outputs.len() != 0,
        Error::OwnerStakeDelegationInvalidTransaction
    )
}
