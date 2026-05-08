use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{close_account, transfer_checked, CloseAccount, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::Escrow;
pub fn handler(ctx: Context<TakeEscrow>) -> Result<()> {
    ctx.accounts.transfer_to_maker()?;
    ctx.accounts.withdraw_and_close_vault()?;
    Ok(())
}

#[account]
pub struct TakeEscroAccount {
    pub taker: Pubkey,   // the person who initiated the take request
    pub maker: Pubkey,   // the person who made the escrow(make.rs),
    escrow: Pubkey,      // the escrow account
    mint_a: Pubkey,      // the mint of the token to be taken
    mint_b: Pubkey,      // the mint of the token to be given
    vault: Pubkey, // vault: the token account associated with the escrow and mint_a that will send the tokens to the taker
    maker_ata_b: Pubkey, // the ata of the token to be given to the maker
}

#[derive(Accounts)]
#[instruction(seed:u64)]
pub struct TakeEscrow<'info> {
    #[account(mut)]
    pub taker: Signer<'info>, // the person who takes the escrow,

    #[account(mut)]
    pub maker: SystemAccount<'info>, //ther person who makes the escrow intially

    #[account(
        mut,
        seeds=[b"escrow",maker.key().as_ref(),seed.to_be_bytes().as_ref()],
        bump=escrow.bump,
        has_one=maker @EscrowError::InvalidMaker,
        has_one=mint_a @EscrowError::InvalidMintA,
        has_one=mint_b @EscrowError::InvalidMintB,
    )]
    pub escrow: Box<Account<'info, Escrow>>, // the account on the contract for escrow
    //token Accounts
    pub mint_a: Box<InterfaceAccount<'info, Mint>>, //the token that the makers deposit
    pub mint_b: Box<InterfaceAccount<'info, Mint>>, //the token that the taker exchange

    #[account(
        mut,
        associated_token::mint=mint_b,
        associated_token::authority=taker,
        associated_token::token_program=token_program
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer=taker,
        associated_token::mint=mint_a,
        associated_token::authority=taker,
        associated_token::token_program=token_program
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint=mint_b,
        associated_token::authority=maker,
        associated_token::token_program=token_program
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>, //
    #[account(
        init_if_needed,
        payer=maker,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    //Programs
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[error_code]
pub enum EscrowError {
    #[msg("The amount to take is not valid")]
    InvalidAmount,
    #[msg("The maker is not valid")]
    InvalidMaker,
    #[msg("Invalid Mint Account.")]
    InvalidMintA,
    InvalidMintB,
}

impl<'info> TakeEscrow<'info> {
    pub fn transfer_to_maker(&mut self) -> Result<()> {
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.taker_ata_b.to_account_info(),
                    to: self.maker_ata_b.to_account_info(),
                    mint: self.mint_b.to_account_info(),
                    authority: self.taker.to_account_info(),
                },
            ),
            self.escrow.receive,
            self.mint_b.decimals,
        )?;
        Ok(())
    }
    pub fn withdraw_and_close_vault(&mut self) -> Result<()> {
        //Create the signer seeds for the vault,

        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.maker.to_account_info().key.as_ref(),
            &self.escrow.seed.to_be_bytes()[..],
            &[self.escrow.bump],
        ]];

        //Transfer Token A Vault-->Taker,

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.vault.to_account_info(),
                    to: self.taker_ata_a.to_account_info(),
                    authority: self.escrow.to_account_info(),
                    mint: self.mint_a.to_account_info(),
                },
                &signer_seeds,
            ),
            self.vault.amount,
            self.mint_a.decimals,
        )?;

        //Close the Vault
        close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                authority: self.escrow.to_account_info(),
                destination: self.maker.to_account_info(),
            },
            &signer_seeds,
        ))?;

        Ok(())
    }
}
