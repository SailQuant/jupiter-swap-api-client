use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use solana_account_decoder::UiAccount;
use solana_sdk::pubkey::Pubkey;

use crate::serde_helpers::option_field_as_string;

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ComputeUnitPriceMicroLamports {
    MicroLamports(u64),
    #[serde(deserialize_with = "auto")]
    Auto,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PriorityLevel {
    Medium,
    High,
    VeryHigh,
}

#[derive(Deserialize, Debug, PartialEq, Copy, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub enum PrioritizationFeeLamports {
    AutoMultiplier(u32),
    JitoTipLamports(u64),
    #[serde(rename_all = "camelCase")]
    PriorityLevelWithMaxLamports {
        priority_level: PriorityLevel,
        max_lamports: u64,
        #[serde(default)]
        global: bool,
    },
    #[default]
    #[serde(untagged, deserialize_with = "auto")]
    Auto,
    #[serde(untagged)]
    Lamports(u64),
    #[serde(untagged, deserialize_with = "disabled")]
    Disabled,
}

impl Serialize for PrioritizationFeeLamports {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct AutoMultiplier {
            auto_multiplier: u32,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct PriorityLevelWrapper<'a> {
            priority_level_with_max_lamports: PriorityLevelWithMaxLamports<'a>,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct PriorityLevelWithMaxLamports<'a> {
            priority_level: &'a PriorityLevel,
            max_lamports: &'a u64,
            global: &'a bool,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct JitoTipLamports {
            jito_tip_lamports: u64,
        }

        match self {
            Self::AutoMultiplier(auto_multiplier) => AutoMultiplier {
                auto_multiplier: *auto_multiplier,
            }
            .serialize(serializer),
            Self::JitoTipLamports(lamports) => JitoTipLamports {
                jito_tip_lamports: *lamports,
            }
            .serialize(serializer),
            Self::Auto => serializer.serialize_str("auto"),
            Self::Lamports(lamports) => serializer.serialize_u64(*lamports),
            Self::Disabled => serializer.serialize_str("disabled"),
            Self::PriorityLevelWithMaxLamports {
                priority_level,
                max_lamports,
                global,
            } => PriorityLevelWrapper {
                priority_level_with_max_lamports: PriorityLevelWithMaxLamports {
                    priority_level,
                    max_lamports,
                    global,
                },
            }
            .serialize(serializer),
        }
    }
}

fn auto<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[serde(rename = "auto")]
        Variant,
    }
    Helper::deserialize(deserializer)?;
    Ok(())
}

fn disabled<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[serde(rename = "disabled")]
        Variant,
    }
    Helper::deserialize(deserializer)?;
    Ok(())
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DynamicSlippageSettings {
    pub min_bps: Option<u16>,
    pub max_bps: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct TransactionConfig {
    /// 包装和解包SOL。如果设置了`destination_token_account`，该选项将被忽略，
    /// 因为`destination_token_account`可能属于其他用户，我们无权关闭该账户。
    pub wrap_and_unwrap_sol: bool,
    /// 允许使用优化的WSOL代币账户方案：通过transfer、assign with seed、allocate with seed
    /// 然后初始化account 3来代替昂贵的关联代币账户创建过程。
    pub allow_optimized_wrapped_sol_token_account: bool,
    /// 输出代币的费用代币账户，通过种子 = ["referral_ata", referral_account, mint] 
    /// 和`REFER4ZgmyYx9c6He5XfaTMiGfdLwRnkV4RPp9t9iF3`推荐合约派生
    /// (仅在设置了feeBps且确保feeAccount已创建时传入)
    #[serde(with = "option_field_as_string")]
    pub fee_account: Option<Pubkey>,
    /// 用于接收交换输出代币的代币账户公钥。如未提供，将使用用户的ATA（关联代币账户）。
    /// 如果提供，我们假定该代币账户已初始化。
    #[serde(with = "option_field_as_string")]
    pub destination_token_account: Option<Pubkey>,
    /// 添加一个只读、非签名的跟踪账户，该账户不被Jupiter使用
    #[serde(with = "option_field_as_string")]
    pub tracking_account: Option<Pubkey>,
    /// 计算单元价格，用于交易优先级排序，额外费用 = 消耗的计算单元 * computeUnitPriceMicroLamports
    pub compute_unit_price_micro_lamports: Option<ComputeUnitPriceMicroLamports>,
    /// 除签名费外，为交易支付的优先级费用（lamports）。
    /// 与`compute_unit_price_micro_lamports`互斥，不可同时使用。
    pub prioritization_fee_lamports: Option<PrioritizationFeeLamports>,
    /// 启用后，将执行交换模拟以获取使用的计算单元，并在ComputeBudget中设置计算单元限制。
    /// 由于需要额外进行一次RPC调用来模拟，这会略微增加延迟。默认为false。
    pub dynamic_compute_unit_limit: bool,
    /// 请求使用传统交易而非默认的版本化交易。需要与使用asLegacyTransaction的报价配对使用，
    /// 否则交易可能过大。
    ///
    /// 默认值: false
    pub as_legacy_transaction: bool,
    /// 启用共享程序账户的使用。这意味着不需要创建中间代币账户或开放订单账户。
    /// 但同时热门账户的可能性也会更高。
    ///
    /// 默认值: 内部优化决定
    pub use_shared_accounts: Option<bool>,
    /// 当交换前的指令包含转账操作从而增加输入代币数量时，此选项非常有用。
    /// 交换将仅使用代币账本记录的数量与当前代币数量之间的差额。
    ///
    /// 默认值: false
    pub use_token_ledger: bool,
    /// 跳过RPC调用，并假设用户账户不存在。
    /// 因此，所有设置指令都会被填充，但不会为用户相关账户（代币账户、Openbook开放订单等）进行RPC调用。
    pub skip_user_accounts_rpc_calls: bool,
    /// 提供带键的UI账户允许加载不在市场缓存中的AMM。
    /// 如果一个带键的UI账户是AMM状态，必须按照市场缓存格式提供其参数。
    pub keyed_ui_accounts: Option<Vec<KeyedUiAccount>>,
    /// 程序授权ID
    pub program_authority_id: Option<u8>,
    /// 动态滑点设置
    pub dynamic_slippage: Option<DynamicSlippageSettings>,
    /// 区块哈希过期前的剩余插槽数
    pub blockhash_slots_to_expiry: Option<u8>,
    /// 请求正确的最后一个有效区块高度，
    /// 这是为了让所有消费者能平滑过渡到agave 2.0，参见 https://github.com/solana-labs/solana/issues/24526
    pub correct_last_valid_block_height: bool,
}


impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            wrap_and_unwrap_sol: true,
            allow_optimized_wrapped_sol_token_account: false,
            fee_account: None,
            destination_token_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: None,
            prioritization_fee_lamports: None,
            as_legacy_transaction: false,
            use_shared_accounts: None,
            use_token_ledger: false,
            dynamic_compute_unit_limit: false,
            skip_user_accounts_rpc_calls: false,
            keyed_ui_accounts: None,
            program_authority_id: None,
            dynamic_slippage: None,
            blockhash_slots_to_expiry: None,
            correct_last_valid_block_height: false,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct KeyedUiAccount {
    pub pubkey: String,
    #[serde(flatten)]
    pub ui_account: UiAccount,
    /// Additional data an Amm requires, Amm dependent and decoded in the Amm implementation
    pub params: Option<Value>,
}
