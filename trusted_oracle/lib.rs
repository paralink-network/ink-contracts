#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod trusted_oracle {
    use ink_storage::collections::{HashMap};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        RequestExpired,
        RequestNotExpired,
        RequestNotFound,
        TransferFailed,
        InsufficientFunds,
        BelowSubsistenceThreshold,
        PaymentRequired,
        CallbackExecutionFailed,
        ValueError,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum OracleResult {
        Numeric(i64),
        RawBytes([u8; 32]),
    }

    #[ink(event)]
    pub struct Request {
        #[ink(topic)]
        from: AccountId,
        /// PQL ETL Definition
        /// Skip first 2 bytes (hash fn, size) so that we can fit into bytes32
        pql_hash: Hash,
        /// Block number for request expiry
        valid_till: u64,
    }

    #[ink(event)]
    pub struct RequestInvalidated {
        #[ink(topic)]
        request_id: u64,
        refunded: Balance,
    }

    #[ink(event)]
    pub struct OracleSet {
        #[ink(topic)]
        oracle: AccountId,
    }

    #[ink(event)]
    pub struct UserAdded {
        #[ink(topic)]
        user: AccountId,
    }

    #[ink(event)]
    pub struct UserRemoved {
        #[ink(topic)]
        user: AccountId,
    }

    #[ink(event)]
    pub struct RewardsClaimed {
        #[ink(topic)]
        oracle: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct FeeChanged {
        #[ink(topic)]
        old_fee: Balance,
        new_fee: Balance,
    }

    #[ink(event)]
    pub struct CallbackComplete {
        #[ink(topic)]
        request_id: u64,
        to: AccountId,
        result: OracleResult
    }

    #[ink(storage)]
    pub struct TrustedOracle {
        /// Admin of the contract
        admin: AccountId,
        /// Who can make the requests
        authorized_users: HashMap<AccountId, ()>,
        /// Who can deliver the results
        authorized_oracle: AccountId,
        /// Store <RequestId, (AccountId, ExpiryBlock, fee)>
        requests: HashMap<u64, (AccountId, u64, Balance)>,
        /// Current request head
        request_idx: u64,
        /// Current fee per request
        fee: Balance,
        /// Minimum number of blocks for request validity
        min_valid_period: u32,
        /// Maximum period for request timeout
        max_valid_period: u32,
    }

    impl TrustedOracle {

        /// Init
        #[ink(constructor)]
        pub fn new(
            admin: AccountId,
            oracle: AccountId,
            min_valid_period: u32,
            max_valid_period: u32) -> Self {
            Self {
                admin: admin,
                authorized_users: HashMap::new(),
                authorized_oracle: oracle,
                requests: HashMap::new(),
                request_idx: 0,
                fee: (0 as u128).into(),
                min_valid_period,
                max_valid_period,
            }
        }

        /// In default case the admin is also the user and the oracle
        #[ink(constructor)]
        pub fn default() -> Self {
            let caller = Self::env().caller();
            let mut authorized_users: HashMap<AccountId,()> = HashMap::new();
            authorized_users.insert(caller, ());
            Self {
                admin: caller,
                authorized_oracle: caller,
                authorized_users,
                requests: HashMap::new(),
                request_idx: 0,
                fee: (0 as u128).into(),
                min_valid_period: 10,
                max_valid_period: 100,
            }
        }

        //
        // User Methods
        //

        /// Make a PQL request
        #[ink(message, payable, selector = "0xB16B00B5")]
        pub fn request(&mut self, pql_hash: Hash, valid_period: u32) -> Result<u64, Error> {
            let from = self.env().caller();

            if !self.authorized_users.contains_key(&from) {
                return Err(Error::Unauthorized);
            }

            if self.fee > (0 as u128).into() {
                if self.env().transferred_balance() != self.fee {
                    return Err(Error::PaymentRequired);
                }
            }

            // loop around to 0 after u64::max_value() is reached
            self.request_idx = self.request_idx.wrapping_add(1);

            // require some reasonable valid_period
            if valid_period < self.min_valid_period ||
               valid_period > self.max_valid_period {
                return Err(Error::ValueError);
            }
            let valid_till = self.env().block_number() + valid_period as u64;
            self.requests.insert(
                self.request_idx,
                (from, valid_till, self.fee),
            );

            self.env().emit_event(Request{from, pql_hash, valid_till});
            Ok(self.request_idx)
        }

        //
        // Oracle Methods
        //

        /// Deliver the oracle result
        #[ink(message)]
        pub fn callback(&mut self,
            request_id: u64,
            callback_addr: AccountId,
            result: OracleResult) -> Result<(),Error> {
            let from = self.env().caller();

            if from != self.authorized_oracle {
                return Err(Error::Unauthorized);
            }

            // check if request_id has expired
            if let Some(request) = self.requests.get(&request_id) {
                let (user_id, valid_till, fee) = request;
                if *valid_till < self.env().block_number() {
                    self.refund_(request_id, *user_id, *fee)?;
                    self.requests.take(&request_id);
                    return Err(Error::RequestExpired);
                }
            } else {
                return Err(Error::RequestNotFound);
            }

            // deliver result as callback
            // note: this will not work off-chain, see:
            // https://paritytech.github.io/ink/src/ink_env/call/call_builder.rs.html#53

            // // method 1:
            // // https://paritytech.github.io/ink/ink_env/fn.invoke_contract.html
            //
            // use ink_env::call::{
            //     utils::{ReturnType},
            //     Selector, ExecutionInput, CallParams};
            // let selector = Selector::new([
            //     0xB1, 0x6B, 0x00, 0xB5,
            // ]);
            // let calldata: CallParams<ink_env::DefaultEnvironment, _, ()> = CallParams{
            //     /// smart contract we are calling
            //     callee: callback_addr,
            //     /// Default gas limit
            //     gas_limit: 1_000_000 as u64,
            //     /// Not sending any funds
            //     transferred_value: (0 as u128).into(),
            //     /// Not expecting a return type
            //     return_type: ReturnType::default(),
            //     /// Function and its args??
            //     exec_input: ExecutionInput::new(selector).push_arg(42)
            // };
            // if let Err(err) = ink_env::invoke_contract(&calldata) {
            //     return Err(Error::CallbackExecutionFailed);
            // }

            // method 2:
            // https://paritytech.github.io/ink/ink_env/call/fn.build_call.html
            //
            use ink_env::call::{build_call, Selector, ExecutionInput};
            let selector = Selector::new([
                0xB1, 0x6B, 0x00, 0xB5,
            ]);
            let callback = build_call::<ink_env::DefaultEnvironment>()
                .callee(callback_addr)
                .gas_limit(1_000_000)
                .transferred_value(0)
                .exec_input(ExecutionInput::new(selector).push_arg(&result))
                .returns::<()>()
                .fire();
            if let Err(_) = callback {
                return Err(Error::CallbackExecutionFailed);
            }

            // TODO
            // There are a few issues with this implementation
            // 1. The callback might not be the same as in PQL.
            // Should the user define the callback in a request instead?
            // 2. Can we do better than responding with raw bytes?
            // Perhaps we could do some decoding here?
            // 3. Should we expect an Ok(()) response from the callee?

            // remove request from storage
            self.requests.take(&request_id);
            let event = CallbackComplete{request_id, to: callback_addr, result};
            self.env().emit_event(event);
            Ok(())
        }


        /// Distribute the rewards to the oracle.
        #[ink(message)]
        pub fn claim_rewards(&mut self) -> Result<(),Error>{
            let from = self.env().caller();

            if from != self.authorized_oracle {
                return Err(Error::Unauthorized);
            }

            // send rewards to the current oracle
            self.claim_()
        }

        //
        // Admin methods
        //

        /// Distribute the rewards to the oracle.
        #[ink(message)]
        pub fn set_oracle(&mut self, new_oracle: AccountId) -> Result<(),Error> {
            let from = self.env().caller();

            if from != self.admin {
                return Err(Error::Unauthorized);
            }

            // send rewards to the current oracle
            self.claim_()?;

            // set new oracle
            self.authorized_oracle = new_oracle;
            self.env().emit_event(OracleSet{oracle: new_oracle});
            Ok(())
        }

        /// Change the per-request fee.
        #[ink(message)]
        pub fn set_fee(&mut self, new_fee: Balance) -> Result<(),Error> {
            let from = self.env().caller();

            if from != self.admin {
                return Err(Error::Unauthorized);
            }

            let old_fee = self.fee.clone();
            self.fee = new_fee;
            self.env().emit_event(FeeChanged{old_fee, new_fee});
            Ok(())
        }


        /// Add user to the oracle contract
        #[ink(message)]
        pub fn add_user(&mut self, user: AccountId) -> Result<(),Error> {
            let from = self.env().caller();

            if from != self.admin {
                return Err(Error::Unauthorized);
            }

            // add the user
            self.authorized_users.insert(user.clone(), ());
            self.env().emit_event(UserAdded{user});
            Ok(())
        }


        /// Remove user from the oracle contract
        #[ink(message)]
        pub fn remove_user(&mut self, user: AccountId) -> Result<(),Error> {
            let from = self.env().caller();

            if from != self.admin {
                return Err(Error::Unauthorized);
            }

            // remove the user
            self.authorized_users.take(&user);
            self.env().emit_event(UserRemoved{user});
            Ok(())
        }

        /// Remove expired request to free contract storage
        #[ink(message)]
        pub fn clear_expired(&mut self, request_id: u64) -> Result<(),Error> {
            if let Some(request) = self.requests.get(&request_id) {
                let (user_id, valid_till, fee) = request;
                if *valid_till < self.env().block_number() {
                    self.refund_(request_id, *user_id, *fee)?;
                    self.requests.take(&request_id);
                    return Ok(());
                } else {
                    return Err(Error::RequestNotExpired);
                }
            } else {
                return Err(Error::RequestNotFound);
            }
        }

        //
        // Other
        //

        // TODO: check if this is private & internal only
        fn claim_(&mut self) -> Result<(),Error> {
            let balance = self.env().balance();
            if balance > (0 as u128).into() {
                let tx = self.env().transfer(self.authorized_oracle, balance);
                return match tx {
                    Ok(_) => {
                        let event = RewardsClaimed{
                            oracle: self.authorized_oracle,
                            amount: balance
                        };
                        self.env().emit_event(event);
                        Ok(())
                    },
                    Err(err) => {
                        match err {
                            ink_env::Error::BelowSubsistenceThreshold =>
                                Err(Error::BelowSubsistenceThreshold),
                            _ => Err(Error::TransferFailed),
                        }
                    }
                }
            }
            Ok(())
        }

        // TODO: check if this is private & internal only
        fn refund_(&mut self, request_id: u64, user_id: AccountId, fee: Balance) -> Result<(),Error> {
            if fee > (0 as u128).into() {
                if self.env().balance() < fee {
                    return Err(Error::InsufficientFunds);
                }
                if let Err(err) = self.env().transfer(user_id, fee) {
                    return match err {
                        ink_env::Error::BelowSubsistenceThreshold =>
                            Err(Error::BelowSubsistenceThreshold),
                        _ => Err(Error::TransferFailed),
                    }
                }
            }
            let event = RequestInvalidated{
                request_id,
                refunded: fee
            };
            self.env().emit_event(event);
            Ok(())
        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        extern crate hex;
        use hex::FromHex;


        #[ink::test]
        fn test_defaults() {
            // alice is admin
            let accounts = default_accounts();
            set_sender(accounts.alice);

            // default contract
            let contract = TrustedOracle::default();
            assert!(contract.admin == accounts.alice);
            assert!(contract.authorized_oracle == accounts.alice);
            assert!(contract.authorized_users.contains_key(&accounts.alice));
        }

        #[ink::test]
        fn test_make_free_request() {
            let mut contract = TrustedOracle::default();
            let pql_hash = sample_ipfs_hash();
            contract.request(pql_hash, 10);
        }


        #[ink::test]
        fn test_make_paid_request() {
            // alice is admin
            let accounts = default_accounts();
            set_sender(accounts.alice);

            // admin sets the fee
            let mut contract = TrustedOracle::default();
            let fee: Balance = (100 as u128).into();
            assert!(contract.set_fee(fee).is_ok());
            assert!(contract.fee == fee);

            let pql_hash = sample_ipfs_hash();

            // payment required
            assert_eq!(contract.request(pql_hash, 10), Err(Error::PaymentRequired));

            // kinda hacky way of sending value into contract
            // assert!(contract.request(pql_hash, 10, {value: 10}).is_ok());
            set_sender(accounts.alice);
            set_balance(accounts.alice, fee);
            let mut data = ink_env::test::CallData::new(ink_env::call::Selector::new([
                0xB1, 0x6B, 0x00, 0xB5,
            ]));
            data.push_arg(&pql_hash);
            data.push_arg(&10);

            // Send "fee" value into the contract
            ink_env::test::push_execution_context::<ink_env::DefaultEnvironment>(
                accounts.alice,
                contract_id(),
                DEFAULT_GAS_LIMIT,
                fee,
                data,
            );
            assert!(contract.request(pql_hash, 10).is_ok());
        }

        #[ink::test]
        fn test_refunds() {
            // alice is admin
            let accounts = default_accounts();
            set_sender(accounts.alice);

            // admin sets the fee
            let mut contract = TrustedOracle::default();
            let fee: Balance = (100 as u128).into();
            assert!(contract.set_fee(fee).is_ok());
            assert!(contract.fee == fee);

            // request is made and paid for
            let pql_hash = sample_ipfs_hash();
            set_sender(accounts.alice);
            set_balance(accounts.alice, fee);
            // TODO: testing if transfer occured is not yet possible
            // in the current version of Ink. Uncomment the get_balance
            // assertions when the ink::test env is fixed.
            // assert_eq!(get_balance(accounts.alice), fee);
            let mut data = ink_env::test::CallData::new(ink_env::call::Selector::new([
                0xB1, 0x6B, 0x00, 0xB5,
            ]));
            data.push_arg(&pql_hash);
            data.push_arg(&10);

            // Send "fee" value into the contract
            ink_env::test::push_execution_context::<ink_env::DefaultEnvironment>(
                accounts.alice,
                contract_id(),
                DEFAULT_GAS_LIMIT,
                fee,
                data,
            );
            // assert_eq!(get_balance(accounts.alice), fee);
            // assert_eq!(get_balance(contract_id()), 0);
            assert!(contract.request(pql_hash, 10).is_ok());
            // assert_eq!(get_balance(contract_id()), fee);
            // assert_eq!(get_balance(accounts.alice), 0);

            // request expires due to non-response
            for _ in 0..10 {
                ink_env::test::advance_block::<ink_env::DefaultEnvironment>().unwrap();
            }

            // request is refunded
            // assert!(contract.clear_expired(1).is_ok());
            // assert_eq!(get_balance(contract_id()), 0);
            // assert_eq!(get_balance(accounts.alice), fee);
        }


        //
        // helper functions
        //
        const DEFAULT_ENDOWMENT: Balance = 1_000_000;
        const DEFAULT_GAS_LIMIT: Balance = 1_000_000;
        fn default_accounts(
        ) -> ink_env::test::DefaultAccounts<ink_env::DefaultEnvironment> {
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                .expect("off-chain environment should have been initialized already")
        }

        fn set_sender(caller: AccountId) {
            ink_env::test::push_execution_context::<ink_env::DefaultEnvironment>(
                caller,
                contract_id(),
                DEFAULT_GAS_LIMIT,
                DEFAULT_ENDOWMENT,
                ink_env::test::CallData::new(ink_env::call::Selector::new([0x00; 4])),
            )
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink_env::test::set_account_balance::<ink_env::DefaultEnvironment>(
                account_id, balance,
            )
                .expect("Cannot set account balance");
        }

        fn get_balance(account_id: AccountId) -> Balance {
            ink_env::test::get_account_balance::<ink_env::DefaultEnvironment>(account_id)
                .expect("Cannot set account balance")
        }

        fn contract_id() -> AccountId {
            ink_env::test::get_current_contract_account_id::<ink_env::DefaultEnvironment>(
            )
                .expect("Cannot get contract id")
         }

        fn sample_ipfs_hash() -> Hash {
            // first 2 bytes omitted
            let input = "42978b1c54ad19f93da7dbc05d0f023062256e95360dfba06c09c1605da75a1b";
            let decoded = <[u8; 32]>::from_hex(input).expect("Decoding failed");
            Hash::from(decoded)
        }

    }

}
