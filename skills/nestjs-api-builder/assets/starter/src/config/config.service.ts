import { Injectable, Logger } from '@nestjs/common';
import { ConfigService as NestConfigService } from '@nestjs/config';
import { TypeOrmModuleOptions } from '@nestjs/typeorm';
import { KafkaConfig, logLevel } from 'kafkajs';

@Injectable()
export class ConfigService extends NestConfigService {
    private readonly logger: Logger = new Logger(ConfigService.name);

    constructor() {
        super();
    }

    getAppPort(): number {
        return Number(this.get('APP_PORT'));
    }

    getAppVersion(): string {
        return this.get('APP_VERSION');
    }

    getDatabaseConfig(): TypeOrmModuleOptions {
        return {
            type: 'postgres',
            schema: this.get('DB_SCHEMA'),
            host: this.get('DB_HOST'),
            port: Number(this.get('DB_PORT')),
            username: this.get('DB_USERNAME'),
            password: this.get('DB_PASSWORD'),
            database: this.get('DB_NAME'),
            migrations: ['dist/migrations/*.js'],
            migrationsTableName: 'migrations',
            logging: ['error'],
            synchronize: false,
            migrationsRun: String(this.get('DB_MIGRATION_RUNS_AUTO')).toLowerCase() === 'true'
        };
    }

    getRedisConfig() {
        return {
            url: this.get('REDIS_URL'),
            socket: {
                reconnectStrategy: (retries: number) => {
                    const delay = Math.min(retries * 1000, 30000);
                    this.logger.log(`Reconnection attempt ${retries}. Retrying in ${delay} ms`);
                    return delay;
                }
            },
            pingInterval: 3000,
            username: this.get('REDIS_USERNAME'),
            password: this.get('REDIS_PASSWORD'),
            database: Number(this.get('REDIS_DATABASE'))
        };
    }

    getKafkaConfig(): KafkaConfig {
        return {
            brokers: [this.get('KAFKA_BROKER')],
            ssl: true,
            sasl: {
                mechanism: 'plain',
                username: this.get('KAFKA_KEY'),
                password: this.get('KAFKA_SECRET')
            },
            clientId: this.get('KAFKA_CLIENT_ID'),
            connectionTimeout: 7000,
            authenticationTimeout: 7000,
            reauthenticationThreshold: 20000,
            logLevel: logLevel.INFO
        };
    }
}

// for code outside DI (migrations)
export const configService = new ConfigService();
