import { Global, Module } from '@nestjs/common';
import { HttpModule as NestHttpModule } from '@nestjs/axios';
import { HttpsProxyAgent } from 'https-proxy-agent';
import { ConfigService } from 'src/config';
import { ConfigModule } from './config.module';

@Global()
@Module({
    imports: [
        NestHttpModule.registerAsync({
            imports: [ConfigModule],
            inject: [ConfigService],
            useFactory: (configService: ConfigService) => (String(configService.get('ENABLE_HTTP_PROXY')).toLowerCase() === 'true' ? {
                httpAgent: new HttpsProxyAgent(configService.get('PROXY_CONNECTION'))
            } : {})
        })
    ],
    exports: [NestHttpModule],
})
export class HttpModule { }
