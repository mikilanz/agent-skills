import { CallHandler, ExecutionContext, HttpException, HttpStatus, Injectable, Logger, NestInterceptor } from '@nestjs/common';
import { Observable, catchError, of } from 'rxjs';
import { DefaultHttpResponseDTO } from 'src/delivery/dto/default.dto';

// Turns every thrown error into a DefaultHttpResponseDTO with ok: false.
// Add a branch in handleError for custom errors that need extra handling (e.g. alerting).
@Injectable()
export class ErrorsInterceptor implements NestInterceptor {
    private readonly logger = new Logger(ErrorsInterceptor.name);

    intercept(context: ExecutionContext, next: CallHandler): Observable<any> {
        return next.handle().pipe(
            catchError((err) => {
                const status = err instanceof HttpException ? err.getStatus() : err?.status || HttpStatus.INTERNAL_SERVER_ERROR;
                const response = context.switchToHttp().getResponse();
                response.status(status);

                return this.handleError(err, status, context);
            })
        );
    }

    private handleError(err: any, status: HttpStatus, context: ExecutionContext): Observable<DefaultHttpResponseDTO> {
        const errorId = this.generateErrorId();
        this.logError(errorId, err, context);

        if (err instanceof HttpException) {
            const response = err.getResponse();
            const message = typeof response === 'string' ? response : response['messages'] || response['message'];
            return this.errorResponse(status, message, errorId);
        }

        return this.errorResponse(status, err?.response?.messages || err?.message, errorId);
    }

    private errorResponse(status: HttpStatus, errors: any, errorId: string): Observable<DefaultHttpResponseDTO> {
        return of(new DefaultHttpResponseDTO({
            statusCode: status,
            errors,
            messages: [`Error ID: ${errorId}`],
            ok: false
        }));
    }

    private logError(errorId: string, err: any, context: ExecutionContext) {
        const request = context.switchToHttp().getRequest();
        this.logger.error(`Error ID ${errorId} - ${err?.message}, Path: ${request.url}`, err?.stack);
    }

    private generateErrorId(): string {
        return Date.now().toString();
    }
}
