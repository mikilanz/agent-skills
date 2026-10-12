import { Logger } from '@nestjs/common';
import { configService } from 'src/config/config.service';
import { MigrationInterface, QueryRunner } from 'typeorm';

export class Init1700000000000 implements MigrationInterface {
    protected readonly logger = new Logger(Init1700000000000.name);
    protected readonly configService = configService;

    public async up(queryRunner: QueryRunner): Promise<void> {
        await queryRunner.query(`CREATE EXTENSION IF NOT EXISTS "uuid-ossp"`);
        await queryRunner.query(`CREATE SCHEMA IF NOT EXISTS ${this.configService.get('DB_SCHEMA')}`);
    }

    public async down(queryRunner: QueryRunner): Promise<void> {
        this.logger.warn('Init migration has no down step');
    }
}
