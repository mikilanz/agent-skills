import { Injectable } from '@nestjs/common';
import { HealthCheckError, HealthIndicator, HealthIndicatorResult } from '@nestjs/terminus';
import { CacheService } from 'src/use-case';

@Injectable()
export class RedisHealthIndicator extends HealthIndicator {
    constructor(private readonly cacheService: CacheService) {
        super();
    }

    async isHealthy(key: string): Promise<HealthIndicatorResult> {
        const isHealthy = await this.cacheService.pingRedis();
        const result = this.getStatus(key, isHealthy, { isHealthy });

        if (isHealthy) {
            return result;
        }
        throw new HealthCheckError('Redis check failed', result);
    }
}
