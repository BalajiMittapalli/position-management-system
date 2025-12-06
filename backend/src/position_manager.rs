use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{Keypair, read_keypair_file},
        signer::Signer,
        system_program,
    },
    Client, Cluster,
};
use anchor_lang::prelude::*;
use anyhow::{anyhow, Result};
use std::{env, rc::Rc, str::FromStr};

// Define the program types based on IDL
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum Side {
    Long,
    Short,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum ModificationType {
    IncreaseSize,
    DecreaseSize,
    AddMargin,
    RemoveMargin,
}

pub struct PositionManager {
    pub client: Client<Rc<Keypair>>,
    pub program_id: Pubkey,
    pub payer: Rc<Keypair>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PositionAccount {
    pub owner: Pubkey,
    pub symbol: String,
    pub side: Side,
    pub size: u64,
    pub entry_price: u64,
    pub margin: u64,
    pub leverage: u8,
    pub unrealized_pnl: i64,
    pub realized_pnl: i64,
    pub funding_accrued: i64,
    pub liquidation_price: u64,
    pub last_update: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UserAccount {
    pub owner: Pubkey,
    pub total_collateral: u64,
    pub locked_collateral: u64,
    pub total_pnl: i64,
    pub position_count: u32,
    pub bump: u8,
}

impl PositionManager {
    pub fn new() -> Result<Self> {
        let rpc_url = env::var("RPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
        
        let program_id_str = env::var("PROGRAM_ID")
            .map_err(|_| anyhow!("PROGRAM_ID environment variable not set"))?;
        
        let wallet_path = env::var("WALLET_PATH")
            .map_err(|_| anyhow!("WALLET_PATH environment variable not set"))?;

        let program_id = Pubkey::from_str(&program_id_str)
            .map_err(|e| anyhow!("Invalid PROGRAM_ID: {}", e))?;

        let payer = read_keypair_file(&wallet_path)
            .map_err(|e| anyhow!("Failed to read wallet file {}: {}", wallet_path, e))?;
        let payer = Rc::new(payer);

        let client = Client::new_with_options(
            Cluster::Custom(rpc_url, "ws://127.0.0.1:8900".to_string()),
            payer.clone(),
            CommitmentConfig::processed(),
        );

        Ok(Self { client, program_id, payer })
    }

    pub fn derive_user_pda(&self, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"user_account", user.as_ref()],
            &self.program_id,
        )
    }

    pub fn derive_position_pda(&self, user: &Pubkey, symbol: &str) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"position", user.as_ref(), symbol.as_bytes()],
            &self.program_id,
        )
    }

    pub fn open_position(
        &self,
        user_pubkey: &Pubkey,
        symbol: &str,
        side: Side,
        size: u64,
        leverage: u8,
        entry_price: u64,
    ) -> Result<String> {
        let (user_pda, _) = self.derive_user_pda(user_pubkey);
        let (position_pda, _) = self.derive_position_pda(user_pubkey, symbol);

        let program = self.client.program(self.program_id)?;

        // Build instruction data
        let instruction_data = [135, 128, 47, 77, 15, 152, 240, 49];
        let accounts = vec![
            anchor_client::solana_sdk::instruction::AccountMeta::new(*user_pubkey, true),
            anchor_client::solana_sdk::instruction::AccountMeta::new(user_pda, false),
            anchor_client::solana_sdk::instruction::AccountMeta::new(position_pda, false),
            anchor_client::solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
        ];

        let mut data = instruction_data.to_vec();
        symbol.serialize(&mut data)?;
        match side {
            Side::Long => 0u8.serialize(&mut data)?,
            Side::Short => 1u8.serialize(&mut data)?,
        }
        size.serialize(&mut data)?;
        leverage.serialize(&mut data)?;
        entry_price.serialize(&mut data)?;

        let instruction = anchor_client::solana_sdk::instruction::Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        let tx = program.request()
            .instruction(instruction)
            .signer(&*self.payer)
            .send()?;

        Ok(tx.to_string())
    }

    pub fn modify_position(
        &self,
        user_pubkey: &Pubkey,
        symbol: &str,
        modification_type: ModificationType,
        amount: u64,
        new_entry_price: Option<u64>,
    ) -> Result<String> {
        let (user_pda, _) = self.derive_user_pda(user_pubkey);
        let (position_pda, _) = self.derive_position_pda(user_pubkey, symbol);

        let program = self.client.program(self.program_id)?;

        let instruction_data = [48, 249, 6, 139, 14, 95, 106, 88];
        let accounts = vec![
            anchor_client::solana_sdk::instruction::AccountMeta::new(*user_pubkey, true),
            anchor_client::solana_sdk::instruction::AccountMeta::new(user_pda, false),
            anchor_client::solana_sdk::instruction::AccountMeta::new(position_pda, false),
        ];

        let mut data = instruction_data.to_vec();
        symbol.serialize(&mut data)?;
        match modification_type {
            ModificationType::IncreaseSize => 0u8.serialize(&mut data)?,
            ModificationType::DecreaseSize => 1u8.serialize(&mut data)?,
            ModificationType::AddMargin => 2u8.serialize(&mut data)?,
            ModificationType::RemoveMargin => 3u8.serialize(&mut data)?,
        }
        amount.serialize(&mut data)?;
        new_entry_price.serialize(&mut data)?;

        let instruction = anchor_client::solana_sdk::instruction::Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        let tx = program.request()
            .instruction(instruction)
            .signer(&*self.payer)
            .send()?;

        Ok(tx.to_string())
    }

    pub fn close_position(
        &self,
        user_pubkey: &Pubkey,
        symbol: &str,
        exit_price: u64,
    ) -> Result<String> {
        let (user_pda, _) = self.derive_user_pda(user_pubkey);
        let (position_pda, _) = self.derive_position_pda(user_pubkey, symbol);

        let program = self.client.program(self.program_id)?;

        let instruction_data = [123, 134, 81, 0, 49, 68, 98, 98];
        let accounts = vec![
            anchor_client::solana_sdk::instruction::AccountMeta::new(*user_pubkey, true),
            anchor_client::solana_sdk::instruction::AccountMeta::new(user_pda, false),
            anchor_client::solana_sdk::instruction::AccountMeta::new(position_pda, false),
        ];

        let mut data = instruction_data.to_vec();
        symbol.serialize(&mut data)?;
        exit_price.serialize(&mut data)?;

        let instruction = anchor_client::solana_sdk::instruction::Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        let tx = program.request()
            .instruction(instruction)
            .signer(&*self.payer)
            .send()?;

        Ok(tx.to_string())
    }

    pub fn get_program_id(&self) -> Pubkey {
        self.program_id
    }

    pub fn get_payer_pubkey(&self) -> Pubkey {
        self.payer.pubkey()
    }
}

impl From<&str> for Side {
    fn from(side: &str) -> Self {
        match side.to_lowercase().as_str() {
            "long" => Side::Long,
            "short" => Side::Short,
            _ => Side::Long,
        }
    }
}

impl From<&str> for ModificationType {
    fn from(mod_type: &str) -> Self {
        match mod_type.to_lowercase().as_str() {
            "increase_size" => ModificationType::IncreaseSize,
            "decrease_size" => ModificationType::DecreaseSize,
            "add_margin" => ModificationType::AddMargin,
            "remove_margin" => ModificationType::RemoveMargin,
            _ => ModificationType::IncreaseSize,
        }
    }
}
