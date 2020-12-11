#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod simple_rng {
    use ink_storage::collections::{HashMap};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        ResultNotFound,
        PermissionDenied,
        DuplicateResult,
        InvalidRequest,
        InvalidResult,
    }

    #[ink(event)]
    pub struct Request {
        #[ink(topic)]
        from: AccountId,
        request_id: u64,
    }

    #[ink(storage)]
    pub struct SimpleRNG {
        owner: AccountId,
        request_id: u64,
        // HashMap<request_id, (min, max)>
        requests: HashMap<u64, (u32, u32)>,
        // HashMap<request_id, randint>
        results: HashMap<u64, u32>
    }

    impl SimpleRNG {

        #[ink(constructor)]
        pub fn new(owner: AccountId) -> Self {
            Self {
                owner: owner,
                request_id: 0,
                requests: HashMap::new(),
                results: HashMap::new(),
            }
        }

        #[ink(constructor)]
        pub fn default() -> Self {
            Self {
                owner: Self::env().caller(),
                request_id: 0,
                requests: Default::default(),
                results: Default::default(),
            }
        }

        #[ink(message)]
        pub fn get_result(&self, request_id: u64) -> Result<u32,Error> {
            if let Some(result) = self.results.get(&request_id) {
                Ok(*result)
            } else {
                Err(Error::ResultNotFound)
            }
        }

        #[ink(message)]
        pub fn make_request(&mut self, min: u32, max: u32) -> Result<u64,Error> {
            let caller = self.env().caller();
            self.request_id += 1;
            self.requests.insert(self.request_id, (min, max));
            self.env().emit_event(Request { from: caller, request_id: self.request_id});
            Ok(self.request_id)
        }

        #[ink(message)]
        pub fn write_result(&mut self, request_id: u64, randint: u32) -> Result<(),Error> {
            let caller = self.env().caller();

            if self.results.contains_key(&request_id) {
                return Err(Error::DuplicateResult);
            }

            let (min, max) = self.requests.get(&request_id).ok_or(Error::InvalidRequest)?;
            if randint < *min || randint > *max {
                return Err(Error::InvalidResult);
            }

            if caller == self.owner {
                self.results.insert(request_id, randint);
            } else {
                return Err(Error::PermissionDenied);
            }
            Ok(())
        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        #[ink::test]
        fn it_sets_owner() {
            let owner = AccountId::from([0x0; 32]);
            let c = SimpleRNG::new(owner);
            assert_eq!(c.owner, owner);
        }

        #[ink::test]
        fn it_makes_new_request() {
            let mut c = SimpleRNG::default();
            assert_eq!(c.make_request(0, 100), Ok(1));
            assert_eq!(c.make_request(0, 100), Ok(2));
            assert_eq!(c.make_request(0, 100), Ok(3));
        }

        #[ink::test]
        fn it_accepts_result() {
            let mut c = SimpleRNG::default();
            let result = 42;
            let request_id = 1;
            assert_eq!(c.make_request(0, 100), Ok(request_id));
            assert_eq!(c.get_result(request_id), Err(Error::ResultNotFound));
            assert_eq!(c.write_result(request_id, result), Ok(()));
            assert_eq!(c.write_result(request_id, result), Err(Error::DuplicateResult));
            assert_eq!(c.get_result(request_id), Ok(result));
        }

        #[ink::test]
        fn it_rejects_result() {
            // alice is admin
            let accounts = default_accounts();
            set_next_caller(accounts.alice);
            let mut c = SimpleRNG::new(accounts.alice);
            assert_eq!(c.owner, accounts.alice);

            let result = 42;
            let request_id = 1;
            assert_eq!(c.make_request(0, 100), Ok(request_id));
            assert_eq!(c.get_result(request_id), Err(Error::ResultNotFound));

            // bob tries to answer
            set_next_caller(accounts.bob);

            assert_eq!(c.write_result(request_id, result), Err(Error::PermissionDenied));
            assert_eq!(c.get_result(request_id), Err(Error::ResultNotFound));
        }


        //
        // helper functions
        //
        const DEFAULT_CALLEE_HASH: [u8; 32] = [0x07; 32];
        const DEFAULT_ENDOWMENT: Balance = 1_000_000;
        const DEFAULT_GAS_LIMIT: Balance = 1_000_000;
        fn default_accounts(
        ) -> ink_env::test::DefaultAccounts<ink_env::DefaultEnvironment> {
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                .expect("off-chain environment should have been initialized already")
        }

        fn set_next_caller(caller: AccountId) {
            ink_env::test::push_execution_context::<ink_env::DefaultEnvironment>(
                caller,
                AccountId::from(DEFAULT_CALLEE_HASH),
                DEFAULT_ENDOWMENT,
                DEFAULT_GAS_LIMIT,
                ink_env::test::CallData::new(ink_env::call::Selector::new([0x00; 4])),
            )
        }

    }
}
