use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

declare_id!("CwEY4zhbE1yVnx6UkKzx64auCGVqVhQD5yMnf526vKmM");

#[program]
pub mod anchor_escrow {
    use super::*;

    const ESCROW_PDA_SEED: &[u8] = b"escrow";
    pub fn initialize(
        _ctx: Context<Initialize>, 
        initialize_amount: u64,
        taker_amount: u64,
    ) -> Result<()> {
        let escrow_account = &mut *_ctx.accounts.escrow_account;
        escrow_account.initializer_key = *_ctx.accounts.initializer.key;
        escrow_account.initializer_deposit_token_account = *_ctx.accounts.initializer_deposit_token_account.to_account_info().key;
        escrow_account.initializer_receive_token_account = *_ctx.accounts.initializer_receive_token_account.to_account_info().key;
        escrow_account.initialize_amount = initialize_amount;
        escrow_account.taker_amount = taker_amount;
        let (pda, _bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], _ctx.program_id);
        token::set_authority(_ctx.accounts.into(), AuthorityType::AccountOwner, Some(pda))?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(initialize_amount: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(
        mut,
        constraint = initializer_deposit_token_account.amount >= initialize_amount
    )]
    pub initializer_deposit_token_account: Account<'info, TokenAccount>,
    pub initializer_receive_token_account: Account<'info, TokenAccount>,
    #[account(init, payer = initializer, space = 8 + EscrowAccount::LEN)]
    pub escrow_account: Account<'info, EscrowAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}


#[account]
pub struct EscrowAccount {
    pub initializer_key: Pubkey,
    pub initializer_deposit_token_account: Pubkey,
    pub initializer_receive_token_account: Pubkey,
    pub initialize_amount: u64,
    pub taker_amount: u64,
}

impl EscrowAccount {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8;
}

