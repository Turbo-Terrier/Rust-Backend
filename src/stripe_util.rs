use std::process::exit;
use std::str::FromStr;
use serde::Serialize;
use stripe::{CheckoutSession, CheckoutSessionMode, Client, Coupon, CreateCheckoutSession, CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData, CreateCoupon, CreateCouponAppliesTo, CreateCustomer, CreatePrice, CreateProduct, CreateProductDefaultPriceData, Currency, Customer, CustomerId, Expandable, ListProducts, Price, PriceId, Product, ProductId, PromotionCode, PromotionCodeCurrencyOption, Timestamp, UpdateCustomer, UpdateProduct};
use crate::data_structs::semester::Semester;
use crate::data_structs::user::User;
use crate::SharedResources;

pub struct StripeHandler {
    stripe_secret_key: String,
    webhook_signing_secret: String,
    stripe_client: Client,
    product_id: ProductId,
    tiered_prices: Vec<TieredPrice>
}

#[derive(Clone, Serialize)]
pub struct TieredPrice {
    required_quantity: u64,
    unit_price: f64
}

impl TieredPrice {
    pub fn new(required_quantity: u64, unit_price: f64) -> TieredPrice {
        return TieredPrice {
            required_quantity,
            unit_price
        };
    }
}

impl Clone for StripeHandler {
    fn clone(&self) -> Self {
        return StripeHandler {
            stripe_secret_key: self.stripe_secret_key.clone(),
            webhook_signing_secret: self.webhook_signing_secret.to_string(),
            stripe_client: Client::new(self.stripe_secret_key.clone()),
            product_id: self.product_id.clone(),
            tiered_prices: self.tiered_prices.clone()
        }
    }
}

impl StripeHandler {

    pub fn new(stripe_secret_key: String, webhook_signing_secret: String, product_id: ProductId, tiered_prices: Vec<TieredPrice>) -> StripeHandler {
        let mut handler =  StripeHandler {
            stripe_secret_key: stripe_secret_key.to_owned(),
            webhook_signing_secret,
            stripe_client: Client::new(stripe_secret_key.to_owned()),
            product_id,
            tiered_prices
        };
        handler.tiered_prices.sort_by(|a, b| a.required_quantity.cmp(&b.required_quantity));
        handler
    }

    pub fn get_tiered_prices(&self) -> &Vec<TieredPrice> {
        return &self.tiered_prices;
    }

    pub fn get_unit_price(&self, quantity: u64) -> f64 {
        let mut price: f64 = -1.0;
        for tiered_price in &self.tiered_prices {
            if quantity >= tiered_price.required_quantity {
                price = tiered_price.unit_price
            }
        }
        if price == -1.0 && self.tiered_prices.len() > 0 {
            price = self.tiered_prices[self.tiered_prices.len() - 1].unit_price;
        }
        assert_ne!(price, -1.0, "Error, unable to figure out the price!"); // should never happen

        return price;
    }

    pub fn get_webhook_signing_secret(&self) -> String {
        return self.webhook_signing_secret.to_owned();
    }

    pub async fn create_new_stripe_customer(&self, customer_full_name: &str, customer_email: &str) -> CustomerId {

        let customer = Customer::create(
            &self.stripe_client,
            CreateCustomer {
                name: Some(customer_full_name),
                email: Some(customer_email),
                ..Default::default()
            },
        )
            .await
            .unwrap();

        customer.id
    }

    pub async fn update_stripe_customer(&self, user: &User) -> CustomerId {

        let full_name = user.given_name.as_str().to_owned() + " " + user.family_name.as_str();
        let stripe_id: CustomerId = user.stripe_id.clone().as_str().parse().unwrap();

        let customer = Customer::update(
            &self.stripe_client,
            &stripe_id,
            UpdateCustomer {
                name: Some(&full_name),
                ..Default::default()
            }
        )
            .await.unwrap();

        return customer.id;
    }

    pub async fn create_stripe_checkout_session(&self, base_url: &String, customer: CustomerId, quantity: u64, unit_price: f64) -> CheckoutSession {

        let redirect_url_success = format!("{}/dashboard?payment_status=success", base_url);
        let redirect_url_failure = format!("{}/dashboard", base_url);
        let mut checkout_session = CreateCheckoutSession::new(redirect_url_success.as_str());
        checkout_session.cancel_url = Option::from(redirect_url_failure.as_str());
        checkout_session.customer = Option::from(customer);
        checkout_session.line_items = Option::from(
            vec!(CreateCheckoutSessionLineItems {
                price_data: Option::from(CreateCheckoutSessionLineItemsPriceData {
                    currency: Currency::USD,
                    product: Option::from(self.product_id.to_string()),
                    unit_amount: Option::from((unit_price * 100.0).round() as i64),
                    ..Default::default()
                }),
                quantity: Option::from(quantity),
                ..Default::default()
            })
        );
        checkout_session.allow_promotion_codes = Option::from(true);
        checkout_session.mode = Option::from(CheckoutSessionMode::Payment);
        let checkout_session = CheckoutSession::create(&self.stripe_client, checkout_session).await.unwrap();

        return checkout_session;
    }

    pub async fn create_coupon(&self, products: Vec<ProductId>, redeem_by: i64, percent_off: f64) {
        let coupon = Coupon::create(
            &self.stripe_client,
            CreateCoupon {
                applies_to: Option::from(CreateCouponAppliesTo {
                    products: Option::from(
                        products.into_iter()
                        .map(|p: ProductId| p.as_str().to_string())
                            .collect::<Vec<String>>()
                    ),
                }),
                currency: Option::from(Currency::USD),
                max_redemptions: Option::from(1),
                percent_off: Option::from(percent_off),
                redeem_by: Option::from(Timestamp::from(redeem_by)),
                ..Default::default()
            }
        ).await.unwrap();
        // todo: no endpoint for promotion codes yet but one is wip
    }

}

