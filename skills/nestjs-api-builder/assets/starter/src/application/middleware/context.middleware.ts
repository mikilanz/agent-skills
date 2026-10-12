import { Injectable, NestMiddleware } from '@nestjs/common';
import { FastifyReply, FastifyRequest } from 'fastify';
import { Context } from 'src/data-provider/entities';

@Injectable()
export class ContextMiddleware implements NestMiddleware {
    use(req: FastifyRequest, res: FastifyReply, next: () => void): any {
        Context.run(req, next);
    }
}
