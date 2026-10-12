import { Global, Module } from '@nestjs/common';
import { CacheService } from 'src/use-case';

@Global()
@Module({
    providers: [CacheService],
    exports: [CacheService]
})
export class CacheModule { }
