use anchor_lang::prelude::*;

use mpl_core:: {
    ID as MPL_CORE_ID,
    fetch_plugin,
    accounts::{BaseAssetV1, BaseCollectionV1}, 
    instructions::{AddPluginV1CpiBuilder, RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder}, 
    types::{Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, PluginType, UpdateAuthority}, 

};
declare_id!("3kn7rbPcVMpAVPdc2q2sykSzxRyWmsYZ7fy9fh64q7MM");

#[program]
pub mod metaplex_core_nft_staking {
    // use mpl_core::{instructions::{AddPluginV1Builder, AddPluginV1CpiBuilder, RemovePluginV1, RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder}, types::{Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, PluginType}};

    use super::*;

    pub fn stake(ctx: Context<Stake>) -> Result<()> {

        match fetch_plugin::<BaseAssetV1, Attributes>(&ctx.accounts.asset.to_account_info(), PluginType::Attributes) {
            Ok((_, fetched_attribute_list, _)) => {
                let mut attribute_list: Vec<Attribute> = vec![];
                let mut is_initialized: bool = false;

                for attribute in fetched_attribute_list.attribute_list {
                    if attribute.key == "staked" {
                        require!(attribute.value == "0", StakingError::AlreadyStaked);
                        is_initialized = true;
                        attribute_list.push(Attribute {
                            key: "staked".to_string(),
                            value: Clock::get()?.unix_timestamp.to_string(),
                        });
                    } else {
                        attribute_list.push(attribute);
                    }
                }

                if !is_initialized {
                    attribute_list.push(Attribute {
                        key: "staked".to_string(),
                        value: Clock::get()?.unix_timestamp.to_string(),});
                    attribute_list.push(Attribute {
                        key: "staked_time".to_string(),
                        value: "0".to_string(),
                    });
                }

                UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
                    .asset(&ctx.accounts.asset.to_account_info())
                    .collection(Some(&ctx.accounts.collection.to_account_info()))
                    .payer(&ctx.accounts.payer.to_account_info())
                    .authority(Some(&ctx.accounts.update_authority.to_account_info()))
                    .system_program(&ctx.accounts.system_program.to_account_info())
                    .plugin(Plugin::Attributes(Attributes {
                        attribute_list,
                    }))
                    .invoke()?;
                
                AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
                    .asset(&ctx.accounts.asset.to_account_info())
                    .collection(Some(&ctx.accounts.collection.to_account_info()))
                    .payer(&ctx.accounts.payer.to_account_info())
                    .authority(Some(&ctx.accounts.update_authority.to_account_info()))
                    .plugin(Plugin::FreezeDelegate(FreezeDelegate {
                        frozen: true,
                    }))
                    .init_authority(PluginAuthority::UpdateAuthority)
                    .invoke()?;

            },
            Err(_) => {},
        }
        Ok(())
    }

    pub fn unstake(ctx: Context<Stake>) -> Result<()> {
        match fetch_plugin::<BaseAssetV1, Attributes>(&ctx.accounts.asset.to_account_info(), PluginType::Attributes) {
            Ok((_ , fetched_attribute_list,_)) => {
                let mut attribute_list: Vec<Attribute> = vec![];
                let mut is_initialized: bool = true;
                let mut staked_time: i64 = 0;

                for attribute in fetched_attribute_list.attribute_list {
                    if attribute.key == "staked" {
                        require!(attribute.value != "0", StakingError::NotStaked);
                        attribute_list.push(Attribute {
                            key: "staked".to_string(),
                            value: "0".to_string(),
                        });

                        staked_time = staked_time.abs()
                            .checked_add(Clock::get()?.unix_timestamp)
                            .ok_or(StakingError::Overflow)?
                            .checked_sub(attribute.value.parse::<i64>().map_err(|_| StakingError::InvalidTimestamp)?)
                            .ok_or(StakingError::Underflow)?;

                    } else if attribute.key == "staked_time" {
                        staked_time = staked_time
                            .checked_add(attribute.value.parse::<i64>().map_err(|_| StakingError::InvalidTimestamp)?)
                            .ok_or(StakingError::Overflow)?;
                    } else {
                        attribute_list.push(attribute);
                    }
                }

                require!(is_initialized, StakingError::NotStaked);

                AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
                    .asset(&ctx.accounts.asset.to_account_info())
                    .collection(Some(&ctx.accounts.collection.to_account_info()))
                    .payer(&ctx.accounts.payer.to_account_info())
                    .authority(Some(&ctx.accounts.update_authority.to_account_info()))
                    .plugin(Plugin::Attributes(Attributes {
                        attribute_list,
                    }))
                    .invoke()?;
            },
            Err(_) => {
                return Err(StakingError::AttributesNotIntialized.into());
            },
        }

        UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.payer.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .plugin(Plugin::FreezeDelegate(FreezeDelegate {
                frozen: false,
            }))
            .invoke()?;
        
        RemovePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.payer.to_account_info())
            .authority(Some(&ctx.accounts.owner.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin_type(PluginType::FreezeDelegate)
            .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Stake<'info> {
    pub owner: Signer<'info>,
    pub update_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        has_one = owner,
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()),
    )]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(
        mut,
        has_one = update_authority
    )]
    pub collection: Account<'info, BaseCollectionV1>,
    
    #[account(
        address = MPL_CORE_ID
    )]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>

}


#[error_code]
pub enum StakingError {
    #[msg("Already staked")]
    AlreadyStaked,

    #[msg("Attributes not initialized")]
    AttributesNotIntialized,

    #[msg("Not staked")]
    NotStaked,

    #[msg("Overflow")]
    Overflow,

    #[msg("Underflow")]
    Underflow,

    #[msg("Invalid timestamp")]
    InvalidTimestamp,

}