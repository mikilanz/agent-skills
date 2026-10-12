import { CallHandler, ExecutionContext, Injectable, Logger, NestInterceptor } from '@nestjs/common';
import { Observable } from 'rxjs';
import { tap } from 'rxjs/operators';

@Injectable()
export class LoggingInterceptor implements NestInterceptor {
    private readonly logger = new Logger(LoggingInterceptor.name);

    intercept(context: ExecutionContext, next: CallHandler<any>): Observable<any> {
        const request = context.switchToHttp().getRequest();
        const { method, url } = request;

        this.logger.log(`Calling endpoint: ${method} ${url}`);

        const now = Date.now();
        return next.handle().pipe(
            tap(() => this.logger.log(`Called endpoint: ${method} ${url} took: ${Date.now() - now}ms`)),
        );
    }
}
