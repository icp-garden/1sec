use alloy::{
    network::{Ethereum, EthereumWallet, TransactionBuilder},
    primitives::{utils::parse_ether, Address, BlockNumber, FixedBytes, U256},
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, Provider, ProviderBuilder, RootProvider,
    },
    rpc::types::{Filter, Log, TransactionReceipt, TransactionRequest},
    signers::local::PrivateKeySigner,
    sol,
};
use alloy_node_bindings::{Anvil, AnvilInstance};
use evm_rpc_types::Nat256;
use one_sec::numeric::Wei;
use reqwest::Url;
use serde_json::json;

pub type EvmProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider,
    Ethereum,
>;

pub struct EvmEnv {
    pub user: Address,
    pub icp: Address,
    pub usdc: Address,
    pub usdc_locker: Address,
    pub provider: EvmProvider,
    pub user_provider: EvmProvider,
    pub anvil_url: Url,
    // Unused, but we need to keep it alive.
    _anvil_instance: AnvilInstance,
}

impl EvmEnv {
    pub async fn new() -> Self {
        let anvil_instance = Anvil::new().try_spawn().unwrap();
        let anvil_url: Url = anvil_instance.endpoint().parse().unwrap();
        let controller_key = anvil_instance.keys()[0].clone();
        let user_addr = anvil_instance.addresses()[1];
        let user_key = anvil_instance.keys()[1].clone();
        let signer: PrivateKeySigner = controller_key.clone().into();
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil_url.clone());

        let user_signer: PrivateKeySigner = user_key.clone().into();
        let user_wallet = EthereumWallet::from(user_signer);
        let user_provider = ProviderBuilder::new()
            .wallet(user_wallet)
            .on_http(anvil_url.clone());

        let icp = Token::deploy(&provider, "ICP".to_string(), 8, U256::from(0))
            .await
            .unwrap();
        let usdc = Token::deploy(&provider, "USDC".to_string(), 6, U256::from(0))
            .await
            .unwrap();
        let usdc_locker = Locker::deploy(&provider, *usdc.address(), U256::from(0))
            .await
            .unwrap();
        Self {
            user: user_addr,
            icp: *icp.address(),
            usdc: *usdc.address(),
            usdc_locker: *usdc_locker.address(),
            provider,
            user_provider,
            anvil_url,
            _anvil_instance: anvil_instance,
        }
    }

    pub async fn token_update_owner(&self, new_owner: Address) {
        Token::new(self.icp, &self.provider)
            .updateOwner(new_owner)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();
    }

    pub async fn locker_update_owner(&self, new_owner: Address) {
        Locker::new(self.usdc_locker, &self.provider)
            .updateOwner(new_owner)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();
    }

    pub async fn usdc_mint(&self, to: Address, amount: Wei) -> TransactionReceipt {
        Token::new(self.usdc, &self.provider)
            .mint(to, U256::try_from(amount.into_inner()).unwrap())
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap()
    }

    pub async fn usdc_deposit(
        &self,
        amount: Wei,
        to_address: FixedBytes<32>,
    ) -> TransactionReceipt {
        let amount = U256::try_from(amount.into_inner()).unwrap();
        Token::new(self.usdc, &self.user_provider)
            .approve(self.usdc_locker, amount)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        Locker::new(self.usdc_locker, &self.user_provider)
            .lock1(amount, to_address)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap()
    }

    pub async fn usdc_transfer(&self, to: Address, amount: Wei) -> TransactionReceipt {
        let amount = U256::try_from(amount.into_inner()).unwrap();
        Token::new(self.usdc, &self.user_provider)
            .transfer(to, amount)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap()
    }

    pub async fn icp_balance_of(&self, account: Address) -> Wei {
        let result = Token::new(self.icp, &self.user_provider)
            .balanceOf(account)
            .call()
            .await
            .unwrap();
        Wei::try_from(Nat256::from_be_bytes(result._0.to_be_bytes())).unwrap()
    }

    pub async fn usdc_balance_of(&self, account: Address) -> Wei {
        let result = Token::new(self.usdc, &self.user_provider)
            .balanceOf(account)
            .call()
            .await
            .unwrap();
        Wei::try_from(Nat256::from_be_bytes(result._0.to_be_bytes())).unwrap()
    }

    pub async fn icp_burn(
        &self,
        contract: Address,
        amount: Wei,
        to_address: FixedBytes<32>,
    ) -> TransactionReceipt {
        Token::new(contract, &self.user_provider)
            .burn1(U256::try_from(amount.into_inner()).unwrap(), to_address)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap()
    }

    pub async fn icp_logs(&self, contract: Address, block: BlockNumber) -> Vec<Log> {
        Token::new(contract, &self.provider);
        let filter = Filter::new()
            .address(contract)
            .from_block(block)
            .to_block(block);
        self.provider.get_logs(&filter).await.unwrap()
    }

    pub async fn transfer_eth(&self, addr: Address, amount: &str) {
        let tx = TransactionRequest::default()
            .with_to(addr)
            .with_value(parse_ether(amount).unwrap());
        self.provider
            .send_transaction(tx)
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();
    }

    pub async fn mine_block(&self) {
        let response: serde_json::Value = self
            .provider
            .client()
            .request("evm_mine", json!({}))
            .await
            .unwrap();
        assert_eq!(response, "0x0");
    }

    pub async fn eth_send_raw_transaction(&self, bytes: &[u8]) -> TransactionReceipt {
        self.provider
            .send_raw_transaction(bytes)
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap()
    }
}

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Token,
    "../contracts/evm/out/Token.sol/Token.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Locker,
    "../contracts/evm/out/Locker.sol/Locker.json"
);
