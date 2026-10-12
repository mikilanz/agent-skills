import { Global, Module } from '@nestjs/common';
import { TypeOrmModule as NestTypeOrmModule } from '@nestjs/typeorm';
import { ConfigService } from 'src/config';
import { ConfigModule } from './config.module';

// Register every entity in BOTH lists.
const entities = [];

@Global()
@Module({
    imports: [
        NestTypeOrmModule.forRootAsync({
            imports: [ConfigModule],
            inject: [ConfigService],
            useFactory: (configService: ConfigService) => ({
                ...configService.getDatabaseConfig(),
                entities: [...entities]
            })
        }),
        NestTypeOrmModule.forFeature([...entities]),
    ],
    exports: [NestTypeOrmModule],
})
export class TypeOrmModule { }
