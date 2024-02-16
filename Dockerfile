# Builder Stage
FROM rust:latest as builder
ENV SQLX_OFFLINE=true

# Create a new Rust project
RUN USER=root cargo new --bin backend
WORKDIR /backend

# Copy and build dependencies
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --locked
RUN rm src/*.rs

# Copy the source code and build the application
COPY . .
RUN cargo build --release --locked

# Production Stage
FROM rust:latest
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /backend/target/release/backend ${APP}/backend

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

ENTRYPOINT ["./backend"]
EXPOSE 8081
