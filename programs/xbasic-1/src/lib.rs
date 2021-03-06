use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

declare_id!("2j4NMzDYQPLpS2HKLR7EnzPt5MXBt3fT9PeWTvUAznQp");

#[program]
pub mod xbasic_1 {
    use super::*;

    pub fn introduce_yourself(ctx: Context<Introduction>, visitor_bump: u8) -> ProgramResult {
        msg!("Nice to meet you {}.", ctx.accounts.visitor.key);
        ctx.accounts.visitor_state.visit_count = 1;
        ctx.accounts.visitor_state.bump = visitor_bump;
        Ok(())
    }

    pub fn visit(ctx: Context<Visit>) -> ProgramResult {
        ctx.accounts.visitor_state.visit_count += 1;
        msg!(
            "Welcome back {}, you've now visited {} times.",
            ctx.accounts.visitor.key,
            ctx.accounts.visitor_state.visit_count
        );
        Ok(())
    }

    pub fn initialize(ctx: Context<Initialize>, data: u64) -> ProgramResult {
        let my_account = &mut ctx.accounts.my_account;
        my_account.data = data;
        Ok(())
    }

    pub fn update(ctx: Context<Update>, data: u64) -> ProgramResult {
        let my_account = &mut ctx.accounts.my_account;
        my_account.data = data;
        Ok(())
    }

    #[access_control(CreateCheck::accounts(&ctx, nonce))]
    pub fn create_check(
        ctx: Context<CreateCheck>,
        amount: u64,
        memo: Option<String>,
        nonce: u8,
    ) -> Result<()> {
        match &memo {
            None => {}
            Some(x) => {
                if &x != &"gm" {
                    return Err(ErrorCode::InvalidMessage.into());
                }
            }
        }
        // Transfer funds to the check.
        let cpi_accounts = Transfer {
            from: ctx.accounts.from.to_account_info().clone(),
            to: ctx.accounts.vault.to_account_info().clone(),
            authority: ctx.accounts.owner.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Print the check.
        let check = &mut ctx.accounts.check;
        check.amount = amount;
        // check.from = *ctx.accounts.from.to_account_info().key;
        // check.to = *ctx.accounts.to.to_account_info().key;
        check.vault = *ctx.accounts.vault.to_account_info().key;
        check.nonce = nonce;
        check.memo = memo;

        Ok(())
    }

    #[access_control(not_burned(&ctx.accounts.check))]
    pub fn cash_check(ctx: Context<CashCheck>) -> Result<()> {
        let seeds = &[
            ctx.accounts.check.to_account_info().key.as_ref(),
            &[ctx.accounts.check.nonce],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info().clone(),
            to: ctx.accounts.to.to_account_info().clone(),
            authority: ctx.accounts.check_signer.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, ctx.accounts.check.amount)?;
        // Burn the check for one time use.
        ctx.accounts.check.burned = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateCheck<'info> {
    // Check being created.
    #[account(zero)]
    check: Account<'info, Check>,
    // Check's token vault.
    #[account(mut, constraint = &vault.owner == check_signer.key)]
    vault: Account<'info, TokenAccount>,
    // Program derived address for the check.
    check_signer: AccountInfo<'info>,
    // Token account the check is made from.
    #[account(mut, has_one = owner)]
    from: Account<'info, TokenAccount>,
    // Token account the check is made to.
    #[account(constraint = from.mint == to.mint)]
    to: Account<'info, TokenAccount>,
    // Owner of the `from` token account.
    owner: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
}

impl<'info> CreateCheck<'info> {
    pub fn accounts(ctx: &Context<CreateCheck>, nonce: u8) -> Result<()> {
        let signer = Pubkey::create_program_address(
            &[ctx.accounts.check.to_account_info().key.as_ref(), &[nonce]],
            ctx.program_id,
        )
        .map_err(|_| ErrorCode::InvalidCheckNonce)?;
        if &signer != ctx.accounts.check_signer.to_account_info().key {
            return Err(ErrorCode::InvalidCheckSigner.into());
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CashCheck<'info> {
    #[account(mut, has_one = vault)]
    check: Account<'info, Check>,
    #[account(mut)]
    vault: AccountInfo<'info>,
    #[account(
        seeds = [check.to_account_info().key.as_ref()],
        bump = check.nonce,
    )]
    check_signer: AccountInfo<'info>,
    #[account(mut, has_one = owner)]
    to: Account<'info, TokenAccount>,
    #[account(signer)]
    owner: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
}

#[account]
pub struct Check {
    from: Pubkey,
    to: Pubkey,
    amount: u64,
    memo: Option<String>,
    vault: Pubkey,
    nonce: u8,
    burned: bool,
}

#[error]
pub enum ErrorCode {
    #[msg("The given nonce does not create a valid program derived address.")]
    InvalidCheckNonce,
    #[msg("The derived check signer does not match that which was given.")]
    InvalidCheckSigner,
    #[msg("The given check has already been burned.")]
    AlreadyBurned,
    #[msg("Sorry that doesn't look like a GM message.")]
    InvalidMessage,
}

fn not_burned(check: &Check) -> Result<()> {
    if check.burned {
        return Err(ErrorCode::AlreadyBurned.into());
    }
    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub my_account: Account<'info, MyAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    pub my_account: Account<'info, MyAccount>,
}

#[account]
pub struct MyAccount {
    pub data: u64,
}

#[derive(Accounts)]
#[instruction(visitor_bump: u8)]
pub struct Introduction<'info> {
    payer: Signer<'info>,
    visitor: Signer<'info>,
    #[account(init, seeds = [visitor.key.as_ref(), "1".as_ref()], bump = visitor_bump, payer = payer, space = 8 + 8 + 1)]
    visitor_state: Account<'info, VisitorState>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Visit<'info> {
    visitor: Signer<'info>,
    #[account(mut, seeds = [visitor.key.as_ref(), "1".as_ref()], bump = visitor_state.bump)]
    visitor_state: Account<'info, VisitorState>,
}

#[account]
pub struct VisitorState {
    visit_count: u64,
    bump: u8,
}
