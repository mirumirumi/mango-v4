use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn perp_cancel_order(ctx: Context<PerpCancelOrder>, order_id: u128) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let perp_market = ctx.accounts.perp_market.load_mut()?;
    let mut book = Orderbook {
        bids: ctx.accounts.bids.load_mut()?,
        asks: ctx.accounts.asks.load_mut()?,
    };

    let (slot, _) = account
        .perp_find_order_with_order_id(perp_market.perp_market_index, order_id)
        .ok_or_else(|| {
            error_msg!("could not find perp order with id {order_id} in user account")
        })?;

    book.cancel_order_by_slot(
        &mut account.borrow_mut(),
        ctx.accounts.account.as_ref().key,
        slot,
        perp_market.perp_market_index,
    )?;

    Ok(())
}
