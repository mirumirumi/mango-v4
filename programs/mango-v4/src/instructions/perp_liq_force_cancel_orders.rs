use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct PerpLiqForceCancelOrders<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,

    #[account(
        mut,
        has_one = group,
        has_one = bids,
        has_one = asks
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,
}

pub fn perp_liq_force_cancel_orders(
    ctx: Context<PerpLiqForceCancelOrders>,
    limit: u8,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_mut()?;

    //
    // Check liqee health if liquidation is allowed
    //
    let mut health_cache = {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let health_cache =
            new_health_cache(&account.borrow(), &retriever).context("create health cache")?;

        if account.being_liquidated() {
            let init_health = health_cache.health(HealthType::Init);
            if account
                .fixed
                .maybe_recover_from_being_liquidated(init_health)
            {
                msg!("Liqee init_health above zero");
                return Ok(());
            }
        } else {
            let maint_health = health_cache.health(HealthType::Maint);
            require!(
                maint_health < I80F48::ZERO,
                MangoError::HealthMustBeNegative
            );
            account.fixed.set_being_liquidated(true);
        }

        health_cache
    };

    //
    // Cancel orders
    //
    {
        let mut perp_market = ctx.accounts.perp_market.load_mut()?;
        let bids = ctx.accounts.bids.load_mut()?;
        let asks = ctx.accounts.asks.load_mut()?;
        let mut book = Book::new(bids, asks);

        book.cancel_all_orders(&mut account.borrow_mut(), &mut perp_market, limit, None)?;

        let perp_position = account.perp_position(perp_market.perp_market_index)?;
        health_cache.recompute_perp_info(perp_position, &perp_market)?;
    }

    //
    // Health check at the end
    //
    let init_health = health_cache.health(HealthType::Init);
    account
        .fixed
        .maybe_recover_from_being_liquidated(init_health);

    Ok(())
}
