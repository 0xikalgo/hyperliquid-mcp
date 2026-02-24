use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};

use crate::state::ServerState;
use crate::tools::{account, market, trading, transfer};

#[derive(Clone)]
pub struct HyperliquidMcp {
    state: ServerState,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl HyperliquidMcp {
    pub fn new(state: ServerState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "get_markets",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_markets(
        &self,
        Parameters(req): Parameters<market::GetMarketsRequest>,
    ) -> Result<CallToolResult, McpError> {
        market::get_markets(&self.state, req).await
    }

    #[tool(
        name = "get_market_summary",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_market_summary(
        &self,
        Parameters(req): Parameters<market::GetMarketSummaryRequest>,
    ) -> Result<CallToolResult, McpError> {
        market::get_market_summary(&self.state, req).await
    }

    #[tool(
        name = "get_order_book",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_order_book(
        &self,
        Parameters(req): Parameters<market::GetOrderBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        market::get_order_book(&self.state, req).await
    }

    #[tool(
        name = "get_candles",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_candles(
        &self,
        Parameters(req): Parameters<market::GetCandlesRequest>,
    ) -> Result<CallToolResult, McpError> {
        market::get_candles(&self.state, req).await
    }

    #[tool(
        name = "get_funding_rates",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_funding_rates(
        &self,
        Parameters(req): Parameters<market::GetFundingRatesRequest>,
    ) -> Result<CallToolResult, McpError> {
        market::get_funding_rates(&self.state, req).await
    }

    #[tool(
        name = "get_wallet_address",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_wallet_address(&self) -> Result<CallToolResult, McpError> {
        account::get_wallet_address(&self.state).await
    }

    #[tool(
        name = "get_positions",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_positions(&self) -> Result<CallToolResult, McpError> {
        account::get_positions(&self.state).await
    }

    #[tool(
        name = "get_balances",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_balances(&self) -> Result<CallToolResult, McpError> {
        account::get_balances(&self.state).await
    }

    #[tool(
        name = "get_open_orders",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_open_orders(
        &self,
        Parameters(req): Parameters<account::GetOpenOrdersRequest>,
    ) -> Result<CallToolResult, McpError> {
        account::get_open_orders(&self.state, req).await
    }

    #[tool(
        name = "get_trade_history",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_trade_history(
        &self,
        Parameters(req): Parameters<account::GetTradeHistoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        account::get_trade_history(&self.state, req).await
    }

    #[tool(
        name = "get_order_status",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_order_status(
        &self,
        Parameters(req): Parameters<account::GetOrderStatusRequest>,
    ) -> Result<CallToolResult, McpError> {
        account::get_order_status(&self.state, req).await
    }

    /// WARNING: Executes a real trade with real funds.
    #[tool(
        name = "place_order",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn place_order(
        &self,
        Parameters(req): Parameters<trading::PlaceOrderRequest>,
    ) -> Result<CallToolResult, McpError> {
        trading::place_order(&self.state, req).await
    }

    #[tool(
        name = "cancel_order",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn cancel_order(
        &self,
        Parameters(req): Parameters<trading::CancelOrderRequest>,
    ) -> Result<CallToolResult, McpError> {
        trading::cancel_order(&self.state, req).await
    }

    #[tool(
        name = "cancel_all_orders",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn cancel_all_orders(
        &self,
        Parameters(req): Parameters<trading::CancelAllOrdersRequest>,
    ) -> Result<CallToolResult, McpError> {
        trading::cancel_all_orders(&self.state, req).await
    }

    #[tool(
        name = "modify_order",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn modify_order(
        &self,
        Parameters(req): Parameters<trading::ModifyOrderRequest>,
    ) -> Result<CallToolResult, McpError> {
        trading::modify_order(&self.state, req).await
    }

    #[tool(
        name = "set_leverage",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn set_leverage(
        &self,
        Parameters(req): Parameters<trading::SetLeverageRequest>,
    ) -> Result<CallToolResult, McpError> {
        trading::set_leverage(&self.state, req).await
    }

    /// WARNING: Immediately closes your full position at market price.
    #[tool(
        name = "close_position",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn close_position(
        &self,
        Parameters(req): Parameters<trading::ClosePositionRequest>,
    ) -> Result<CallToolResult, McpError> {
        trading::close_position(&self.state, req).await
    }

    #[tool(
        name = "schedule_cancel",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn schedule_cancel(
        &self,
        Parameters(req): Parameters<trading::ScheduleCancelRequest>,
    ) -> Result<CallToolResult, McpError> {
        trading::schedule_cancel(&self.state, req).await
    }

    #[tool(
        name = "transfer_between_spot_perps",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn transfer_between_spot_perps(
        &self,
        Parameters(req): Parameters<transfer::TransferSpotPerpsRequest>,
    ) -> Result<CallToolResult, McpError> {
        transfer::transfer_between_spot_perps(&self.state, req).await
    }

    #[tool(
        name = "create_agent_wallet",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn create_agent_wallet(&self) -> Result<CallToolResult, McpError> {
        transfer::create_agent_wallet(&self.state).await
    }

    #[tool(
        name = "approve_builder_fee",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn approve_builder_fee(&self) -> Result<CallToolResult, McpError> {
        transfer::approve_builder_fee(&self.state).await
    }

    #[tool(
        name = "check_builder_fee",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn check_builder_fee(&self) -> Result<CallToolResult, McpError> {
        transfer::check_builder_fee(&self.state).await
    }
}

#[tool_handler]
impl ServerHandler for HyperliquidMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Hyperliquid MCP Server — trade perpetual futures and spot assets on Hyperliquid. \
                 Use market data tools to check prices, order books, and funding rates. \
                 Use account tools to view positions, balances, and trade history. \
                 Use trading tools to place, modify, and cancel orders. \
                 WARNING: Trading tools execute real trades with real funds. \
                 Always confirm trade details with the user before executing."
                    .to_string(),
            ),
        }
    }
}
