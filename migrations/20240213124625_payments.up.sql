CREATE TABLE IF NOT EXISTS payments (
    id UUID PRIMARY KEY NOT NULL DEFAULT (uuid_generate_v4()),
    name VARCHAR(255) NOT NULL,
    description VARCHAR(510) NOT NULL UNIQUE,
    user_id UUID NOT NULL,
    category_id UUID NOT NULL,
    price DOUBLE PRECISION,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT fk_user FOREIGN KEY(user_id) REFERENCES users(id),
    CONSTRAINT fk_category FOREIGN KEY(category_id) REFERENCES categories(id)
);