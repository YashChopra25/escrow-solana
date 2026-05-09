use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{close_account, transfer_checked, CloseAccount, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::Escrow;

pub fn handler(ctx: Context<RefundFn>,seed:u64) -> Result<()> {
    ctx.accounts.transfer_to_maker(seed)?;
    Ok(())
}

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct RefundFn<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        seeds=[b"escrow",maker.key().as_ref(),seed.to_be_bytes().as_ref()],
        bump=escrow.bump,
        has_one=maker
    )]
    pub escrow: Box<Account<'info, Escrow>>,
    #[account(
        mint::token_program=token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint=mint_a,
        associated_token::authority=maker,
        associated_token::token_program=token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint=mint_a,
        associated_token::authority=escrow,
        associated_token::token_program=token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    //programs
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> RefundFn<'info> {
    pub fn transfer_to_maker(&mut self, seed: u64) -> Result<()> {
        let signer_seed: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.maker.key.as_ref(),
            &seed.to_be_bytes()[..],
            &[self.escrow.bump],
        ]];
        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.vault.to_account_info(),
                    to: self.maker.to_account_info(),
                    authority: self.escrow.to_account_info(),
                    mint: self.mint_a.to_account_info(),
                },
                &signer_seed,
            ),
            self.vault.amount,
            self.mint_a.decimals,
        )?;

        close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                authority: self.escrow.to_account_info(),
                destination: self.maker.to_account_info(),
            },
            &signer_seed,
        ))?;
        Ok(())
    }
}

#[error_code]
pub enum RefundErr {
    #[msg("You are not authorized to take refund")]
    InvalidOwner,
}
