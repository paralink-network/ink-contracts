#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod oracle_requester_consumer {

    /// We add the type with currently supported Oracle results
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum OracleResult {
        Numeric(i64),
        RawBytes([u8; 32]),
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        OracleRequestError,
    }

    #[ink(storage)]
    pub struct OracleRequesterConsumer {
        /// The smart contract of the Oracle we are inherently trusting
        /// with providing the data feeds
        authorized_oracle: AccountId,
        /// This is the value we will be updating trough the oracle
        /// It does not have to be the same size as `OracleResult::Numeric`
        bitcoin_price: u64,
        /// Admin of this contract.
        admin: AccountId,
    }

    impl OracleRequesterConsumer {

        #[ink(constructor)]
        pub fn new(
            authorized_oracle: AccountId,
            bitcoin_price: u64,
            admin: AccountId) -> Self {
            // set the oracle which will be allowed to update our bitcoin price
            // set the intial price on contract creation
            // set the admin
            Self {
                authorized_oracle,
                bitcoin_price,
                admin,
            }
        }


        /// This method is used to request the work from the Oracle.
        /// Note that the `OracleRequesterConsumer` contract needs to be
        /// whitelisted as authorized_user on the Oracle in order to make the job requests.
        ///
        /// In principle your smart contract does not need to be an originator of a request.
        /// If you need only to recieve results into your smart contract, check `OracleConsumer`.
        #[ink(message, payable)]
        pub fn request_oracle_update(&mut self, pql: Hash, valid_period: u32) -> Result<(),Error> {
            // only admin can request an oracle job
            // to avoid this requirement, you can:
            //  - pre-fund the contract with sufficent balance to pay for fees
            //  - make pql_hash and valid_period part of self.()
            let who = self.env().caller();
            if who != self.admin {
                return Err(Error::Unauthorized);
            }

            // the amount sent to this call will be forwarded to the oracle to pay the fee
            let fee = self.env().transferred_balance();

            // request data from our oracle
            use ink_env::call::{build_call, Selector, ExecutionInput};
            let selector = Selector::new([
                0xB1, 0x6B, 0x00, 0xB5,
            ]);
            let request = build_call::<ink_env::DefaultEnvironment>()
                .callee(self.authorized_oracle)
                .gas_limit(1_000_000)
                .transferred_value(fee)
                .exec_input(ExecutionInput::new(selector)
                    .push_arg(&pql)
                    .push_arg(&valid_period))
                .returns::<()>()
                .fire();
            if let Err(_) = request {
                return Err(Error::OracleRequestError);
            }

            Ok(())
        }

        /// This method is called from the Oracle's `callback` fn.
        /// It can be named anything (in this case `set_bitcoin_price`),
        /// however it does need a fixed selector.
        /// The selector value needs to be the same as in the Oracle contract.
        #[ink(message, selector = "0xB16B00B5")]
        pub fn set_bitcoin_price(&mut self, result: OracleResult) -> Result<(),Error> {
            // check if the oracle is trusted
            let oracle = self.env().caller();
            if oracle != self.authorized_oracle {
                return Err(Error::Unauthorized);
            }

            // set the oracle's value
            if let OracleResult::Numeric(price) = result {
                self.bitcoin_price = price as u64;
            }

            // Let the oracle know all is good
            Ok(())
        }

        /// Meme function. Note that since smart contracts don't support
        /// floats, we deliberately encoded 8 decimal points of precision
        /// by using large ints.
        #[ink(message)]
        pub fn its_over_9000(&self) -> bool {
            self.bitcoin_price > 9000 as u64 * 1e8 as u64
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn default_works() {
            let oracle_stub: AccountId = [0x0; 32].into();
            let admin_stub: AccountId = [0x0; 32].into();
            let c = OracleRequesterConsumer::new(oracle_stub, 0, admin_stub);
            assert_eq!(c.its_over_9000(), false);
        }

    }
}
