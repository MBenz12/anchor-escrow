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

    pub fn exchange(_ctx: Context<Exchange>) -> Result<()> {
        let (_pda, bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], _ctx.program_id);
        let seeds = &[&ESCROW_PDA_SEED[..], &[bump_seed]];
        token::transfer(
            _ctx.accounts
                .into_transfer_to_taker_context()
                .with_signer(&[&seeds[..]]),
            _ctx.accounts.escrow_account.initialize_amount
        )?;

        token::transfer(
            _ctx.accounts.into_transfer_to_initializer_context(),
            _ctx.accounts.escrow_account.taker_amount
        )?;

        token::set_authority(
            _ctx.accounts
                .into_set_authority_context()
                .with_signer(&[&seeds[..]]),
            AuthorityType::AccountOwner,
            Some(_ctx.accounts.escrow_account.initializer_key),
        )?;
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

#[derive(Accounts)]
pub struct Exchange<'info> {
    /// CHECK:
    #[account(signer)]
    pub taker: AccountInfo<'info>,
    #[account(mut)]
    pub taker_deposit_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub taker_receive_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pda_deposit_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub initializer_receive_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    /// CHECK:
    pub initializer_main_account: AccountInfo<'info>,
    #[account(
        mut,
        constraint = escrow_account.taker_amount <= taker_deposit_token_account.amount,
        constraint = escrow_account.initializer_deposit_token_account == *pda_deposit_token_account.to_account_info().key,
        constraint = escrow_account.initializer_receive_token_account == *initializer_receive_token_account.to_account_info().key,
        constraint = escrow_account.initializer_key == *initializer_main_account.key,
        close = initializer_main_account
    )]
    pub escrow_account: Account<'info, EscrowAccount>,
    /// CHECK:
    pub pda_account: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

impl<'info> Exchange<'info> {
    fn into_transfer_to_taker_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.pda_deposit_token_account.to_account_info().clone(),
            to: self.taker_receive_token_account.to_account_info().clone(),
            authority: self.pda_account.clone()
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    fn into_transfer_to_initializer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.taker_deposit_token_account.to_account_info().clone(),
            to: self.initializer_receive_token_account.to_account_info().clone(),
            authority: self.taker.clone()
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    fn into_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.pda_deposit_token_account.to_account_info().clone(),
            current_authority: self.pda_account.clone(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
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

