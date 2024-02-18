use std::{error::Error, sync::Arc};
use futures::StreamExt;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicRejectOptions,
        ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
    },
    types::{AMQPValue, FieldTable},
    Connection, ConnectionProperties,
};
use serde_json::from_slice;
use crate::{
    amqp::{config::get_config, dto::PaymentMessage},
    payment::model::CreatePaymentSchema,
    payment::handler::create_payment_handler
};
use actix_web::{web, HttpResponse};

pub async fn run(pg_pool: Arc<Pool>) -> Result<(), Box<dyn Error>> {
    let config = get_config();

    let connection = Arc::new(Connection::connect(&config.amqp_addr, ConnectionProperties::default()).await?);

    connection.on_error(|err| {
        log::error!("{}", err);
        std::process::exit(1);
    });

    let declare_channel = connection.create_channel().await?;

    setup_dead_letter_exchange(&declare_channel).await?;
    setup_payments_exchange(&declare_channel).await?;
    setup_payments_queue(&declare_channel).await?;
    declare_channel.close(0, "declare channel fineshed").await?;

    listen_for_payments(consumer_channel).await?;

    Ok(())
}

async fn setup_dead_letter_exchange(declare_channel: &lapin::Channel) -> Result<(), lapin::Error> {
    declare_channel
        .exchange_declare(
            "payments-dead-letter.exchange",
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    declare_channel
        .queue_declare(
            "payments-dead-letter.queue",
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    declare_channel
        .queue_bind(
            "payments-dead-letter.queue",
            "payments-dead-letter.exchange",
            "",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    Ok(())
}

async fn setup_payments_exchange(declare_channel: &lapin::Channel) -> Result<(), lapin::Error> {
    declare_channel
        .exchange_declare(
            "payments.exchange",
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    Ok(())
}

async fn setup_payments_queue(declare_channel: &lapin::Channel) -> Result<(), lapin::Error> {
    let mut queue_field = FieldTable::default();
    queue_field.insert(
        "x-dead-letter-exchange".into(),
        AMQPValue::LongString("payments-dead-letter.exchange".into()),
    );

    declare_channel
        .queue_declare(
            "payments.queue",
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            queue_field,
        )
        .await?;

    declare_channel
        .queue_bind(
            "payments.queue",
            "payments.exchange",
            "",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    Ok(())
}

async fn listen_for_payments(consumer_channel: lapin::Channel) -> Result<(), lapin::Error> {
    let mut consumer = consumer_channel
        .basic_consume(
            "payments.queue",
            "consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    log::info!("Server listening to payments.queue");

    while let Some(result) = consumer.next().await {
        if let Ok(delivery) = result {
            match process_payment_message(delivery.data.as_slice()).await {
                Ok(_) => delivery.ack(BasicAckOptions::default()).await?,
                Err(err) => {
                    log::error!("Failed to process payment message: {}", err);
                    delivery.nack(BasicNackOptions::default()).await?;
                }
            }
        }
    }

    Ok(())
}

async fn process_payment_message(data: &[u8]) -> Result<(), Box<dyn Error>> {
    let payment_message: PaymentMessage = from_slice(data)?;

    let create_payment_schema = CreatePaymentSchema {
        name: payment_message.name,
        description: payment_message.description.unwrap_or_default(),
        price: payment_message.price.unwrap_or_default(),
        userId: payment_message.userId.unwrap_or_default(),
        categoryId: payment_message.categoryId.unwrap_or_default(),
    };

    let response = create_payment_handler(web::Json(create_payment_schema)).await;
    match response {
        Ok(http_response) => {
            if http_response.status().is_success() {
                Ok(())
            } else {
                Err("Failed to create payment".into())
            }
        }
        Err(_) => Err("Failed to create payment".into()),
    }
}
