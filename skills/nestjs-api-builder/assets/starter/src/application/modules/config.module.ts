import { Global, Module } from '@nestjs/common';
import { ConfigModule as NestConfigModule } from '@nestjs/config';
import { ConfigService, validate } from 'src/config';

@Global()
@Module({
    imports: [
        NestConfigModule.forRoot({
            cache: true,
            isGlobal: true,
            validate
        })
    ],
    providers: [ConfigService],
    exports: [ConfigService],
})
export class ConfigModule { }
