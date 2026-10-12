import { CallHandler, ExecutionContext, Injectable, NestInterceptor } from '@nestjs/common';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { FastifyReply } from 'fastify';
import { CustomResponseDTO } from 'src/delivery/dto/custom-response.dto';

// The handler must return { data, headers }: headers go on the reply, data becomes the body.
@Injectable()
export class HeadersInterceptor implements NestInterceptor {
    intercept(context: ExecutionContext, next: CallHandler<any>): Observable<any> {
        const response: FastifyReply = context.switchToHttp().getResponse();
        return next.handle().pipe(
            map((res: CustomResponseDTO<any>) => {
                if (res?.headers) {
                    Object.keys(res.headers).forEach(header => {
                        response.header(header, res.headers[header]);
                    });
                }
                return res?.data;
            }),
        );
    }
}
