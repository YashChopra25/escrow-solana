use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

const ANCHOR_DISCRIMINATOR: usize = 8;

pub fn handler(ctx: Context<MakeEscrow>, seed: u64, receive: u64, amount: u64) -> Result<()> {
    require_gt!(receive, 0, EscrowError::InvalidAmount);
    require_gt!(amount, 0, EscrowError::InvalidAmount);
    ctx.accounts
        .populate_escrow(seed, receive, ctx.bumps.escrow)?;
    ctx.accounts.deposit_token(amount)?;
    Ok(())
}

#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub receive: u64,
    pub bump: u8,
}

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct MakeEscrow<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        init,
        space = ANCHOR_DISCRIMINATOR + Escrow::INIT_SPACE,
        payer = maker,
        seeds = [b"escrow", maker.key().as_ref(), seed.to_be_bytes().as_ref()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(mint::token_program = token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    //programs
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> MakeEscrow<'info> {
    fn populate_escrow(&mut self, seed: u64, receive: u64, bump: u8) -> Result<()> {
        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive,
            bump,
        });

        Ok(())
    }

    fn deposit_token(&mut self, amount: u64) -> Result<()> {
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.maker_ata_a.to_account_info(),
                    mint: self.mint_a.to_account_info(),
                    to: self.vault.to_account_info(),
                    authority: self.maker.to_account_info(),
                },
            ),
            amount,
            self.mint_a.decimals,
        )?;
        Ok(())
    }
}

#[error_code]
pub enum EscrowError {
    #[msg("The enter amount is not valid")]
    InvalidAmount,
}
