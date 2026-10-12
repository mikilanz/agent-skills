import * as Joi from 'joi';

type Config = Record<string, any>;

export function validate(configs: Config): Config {
    const validationSchema: Joi.ObjectSchema = Joi.object({
        NODE_ENV: Joi.string()
            .valid('development', 'staging', 'production')
            .default('development'),
        APP_PORT: Joi.number().default(8080),
        APP_VERSION: Joi.string().default('0.1.0'),

        // === Database ===
        DB_SCHEMA: Joi.string().default('public'),
        DB_HOST: Joi.string().required(),
        DB_PORT: Joi.number().default(5432),
        DB_USERNAME: Joi.string().required(),
        DB_PASSWORD: Joi.string().required(),
        DB_NAME: Joi.string().required(),
        DB_MIGRATION_RUNS_AUTO: Joi.bool().default(false),

        // === Proxy ===
        ENABLE_HTTP_PROXY: Joi.bool().default(false),
        PROXY_CONNECTION: Joi.string().allow('').default(''),

        // === Redis ===
        REDIS_URL: Joi.string().required(),
        REDIS_USERNAME: Joi.string().allow('').default(''),
        REDIS_PASSWORD: Joi.string().allow('').default(''),
        REDIS_DATABASE: Joi.number().default(0),

        // === Kafka ===
        KAFKA_BROKER: Joi.string().required(),
        KAFKA_KEY: Joi.string().required(),
        KAFKA_SECRET: Joi.string().required(),
        KAFKA_CLIENT_ID: Joi.string().required(),
    }).unknown();

    const { error, value } = validationSchema.validate(configs);
    if (error) throw new Error(`Configuration validation failed: ${error.message}`);
    return value;
}
