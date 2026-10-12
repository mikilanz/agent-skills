import { Injectable, Logger, OnModuleDestroy, OnModuleInit } from '@nestjs/common';
import { createClient, RedisClientType } from 'redis';
import { ConfigService } from 'src/config';

@Injectable()
export class CacheService implements OnModuleInit, OnModuleDestroy {
    protected readonly logger: Logger = new Logger(CacheService.name);
    protected readonly redisClient: RedisClientType;

    constructor(private readonly configService: ConfigService) {
        this.redisClient = createClient(this.configService.getRedisConfig());

        this.redisClient.on('error', (err) => {
            this.logger.error(`Redis error: ${err.code ?? err}`);
        });
    }

    async onModuleInit(): Promise<void> {
        this.redisClient.on('connect', () => this.logger.log('Connected to Redis'));
        this.redisClient.on('reconnecting', () => this.logger.log('Reconnecting to Redis'));

        try {
            await this.redisClient.connect();
        } catch (err) {
            this.logger.error(`Error connecting to Redis: ${err.message}`);
        }
    }

    async onModuleDestroy(): Promise<void> {
        try {
            await this.redisClient.quit();
        } catch (err) {
            this.logger.error(`Error quitting Redis: ${err.message}`);
        }
    }

    async get<T>(key: string): Promise<T | null> {
        try {
            const value = await this.redisClient.GET(key);
            return value ? JSON.parse(value) as T : null;
        } catch (err) {
            this.logger.error(`Error retrieving key '${key}' from Redis: ${err.message}`);
            return null;
        }
    }

    async set<T>(key: string, value: T, ttl?: number): Promise<void> {
        try {
            const serializedValue = JSON.stringify(value);
            if (ttl) {
                await this.redisClient.SET(key, serializedValue, { EX: ttl });
            } else {
                await this.redisClient.SET(key, serializedValue);
            }
        } catch (err) {
            this.logger.error(`Error setting key '${key}' in Redis: ${err.message}`);
        }
    }

    async delete(key: string): Promise<void> {
        try {
            await this.redisClient.DEL(key);
        } catch (err) {
            this.logger.error(`Error deleting key '${key}' from Redis: ${err.message}`);
        }
    }

    async exists(key: string): Promise<boolean> {
        try {
            return (await this.redisClient.EXISTS(key)) === 1;
        } catch (err) {
            this.logger.error(`Error checking existence of key '${key}' in Redis: ${err.message}`);
            return false;
        }
    }

    async pingRedis(): Promise<boolean> {
        try {
            return (await this.redisClient.ping()) === 'PONG';
        } catch (err) {
            this.logger.error(`Error pinging Redis: ${err.message}`);
            return false;
        }
    }
}
