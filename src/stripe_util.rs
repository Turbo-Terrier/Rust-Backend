use std::process::exit;
use std::str::FromStr;
use stripe::{CheckoutSession, CheckoutSessionMode, Client, Coupon, CreateCheckoutSession, CreateCheckoutSessionLineItems, CreateCoupon, CreateCouponAppliesTo, CreateCustomer, CreatePrice, CreateProduct, CreateProductDefaultPriceData, Currency, Customer, CustomerId, Expandable, ListProducts, Price, PriceId, Product, ProductId, PromotionCode, PromotionCodeCurrencyOption, Timestamp, UpdateCustomer, UpdateProduct};
use crate::data_structs::semester::Semester;
use crate::data_structs::user::User;
use crate::SharedResources;

pub struct StripeHandler {
    stripe_secret_key: String,
    webhook_signing_secret: String,
    stripe_client: Client,
    regular_base_price: i64,
    summer_base_price: i64
}

impl Clone for StripeHandler {
    fn clone(&self) -> Self {
        return StripeHandler {
            stripe_secret_key: self.stripe_secret_key.clone(),
            webhook_signing_secret: self.webhook_signing_secret.to_string(),
            stripe_client: Client::new(self.stripe_secret_key.clone()),
            regular_base_price: self.regular_base_price,
            summer_base_price: self.summer_base_price,
        }
    }
}

impl StripeHandler {

    pub fn new(stripe_secret_key: String, webhook_signing_secret: String, regular_base_price: i64, summer_base_price: i64) -> StripeHandler {
        return StripeHandler {
            stripe_secret_key: stripe_secret_key.to_owned(),
            webhook_signing_secret: webhook_signing_secret.to_owned(),
            stripe_client: Client::new(stripe_secret_key.to_owned()),
            regular_base_price: regular_base_price,
            summer_base_price: summer_base_price,
        }
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


    //todo : rm
	// https://github.com/arlyon/async-stripe/blob/master/examples
    pub async fn create_or_get_products(&self, shared_resources: &SharedResources) -> Vec<Product> {

        let normal_base_price = shared_resources.stripe_handler.regular_base_price;
        let summer_base_price = shared_resources.stripe_handler.summer_base_price;

        let mut semesters_to_sell: Vec<String> = Semester::get_current_and_upcoming_semesters()
            .into_iter().map(| sem: Semester | sem.to_string()).collect();

        let mut added_semesters: Vec<String> = Vec::new();

        let mut products = Product::list(
            &self.stripe_client,
            &Default::default()
        ).await.unwrap().data;

        for product in &products {
            if product.active.unwrap() && semesters_to_sell.contains(&product.name.clone().unwrap()) {
                added_semesters.push(product.name.clone().unwrap());
            } else {
                // deactivate otherwise
                Product::update(&self.stripe_client, &product.id, UpdateProduct {
                    active: Option::from(false),
                    ..Default::default()
                }).await.expect("Error updating product!");
            }
        };

        let mut redirect_url = shared_resources.base_url.clone();
        redirect_url.push_str("/api/web/v1/payment-status");
        for semester in semesters_to_sell {
            if !added_semesters.contains(&semester) {

                let base_price = {
                    let semester: Semester = match Semester::from_str(semester.as_str()) {
                        Ok(semester) => semester,
                        Err(e) => {
                            eprintln!("Error parsing semester {}: {}", semester, e);
                            continue;
                        }
                    };
                    if semester.semester_season.is_summer_session() {
                        normal_base_price.clone()
                    } else {
                        summer_base_price.clone()
                    }
                };

                println!("{:#?}", Option::from(("TT: ".to_string() + &semester.to_string()).as_str())); //todo remove
                let product = Product::create(
                    &self.stripe_client,
                    CreateProduct {
                        active: None,
                        default_price_data: Option::from(
                            CreateProductDefaultPriceData {
                                currency: Currency::USD,
                                unit_amount: Option::from(base_price),
                                ..Default::default()
                            }
                        ),
                        description: Option::from(
                            ("Purchase unlimited premium access to the Turbo Terrier app for any registrations ".to_string() +
                                "for the " + &semester.to_string() + " semester.").as_str()
                        ),
                        expand: &[],
                        id: None,
                        images: None,
                        metadata: None,
                        name: semester.to_string().as_str(),
                        package_dimensions: None,
                        shippable: None,
                        statement_descriptor: Option::from(("TT: ".to_string() + &semester.to_string()).as_str()),
                        tax_code: None,
                        type_: None,
                        unit_label: None,
                        url: None,
                    }
                ).await.unwrap();

                products.push(product);
            }
        }

        //todo sort by semester
        products
    }

    pub async fn create_stripe_checkout_session(&self, base_url: &String, customer: CustomerId, product_price_id: &PriceId) -> CheckoutSession {

        let redirect_url_success = format!("{}/api/web/v1/payment-status/success", base_url);
        let redirect_url_failure = format!("{}/api/web/v1/payment-status/failure", base_url);
        let mut checkout_session = CreateCheckoutSession::new(redirect_url_success.as_str());
        checkout_session.cancel_url = Option::from(redirect_url_failure.as_str());
        checkout_session.customer = Option::from(customer);
        checkout_session.line_items = Option::from(
            vec!(CreateCheckoutSessionLineItems {
                price: Option::from(product_price_id.to_string()), //also has product details
                quantity: Option::from(1),
                ..Default::default()
            })
        );
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

