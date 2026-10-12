import { AsyncLocalStorage } from 'async_hooks';
import { FastifyRequest } from 'fastify';
import { Claim } from 'src/delivery/dto/claim.dto';

// Per-request holder, set by ContextMiddleware. AsyncLocalStorage keeps concurrent
// requests isolated, so non-request code (subscribers, audit logging) can read the caller.
export class Context {
    private static readonly storage = new AsyncLocalStorage<FastifyRequest>();

    public static run(req: FastifyRequest, next: () => void) {
        this.storage.run(req, next);
    }

    private static get req(): FastifyRequest {
        return this.storage.getStore();
    }

    public static getUser(): Claim {
        try {
            const userinfo =
                this.req.headers['x-apigateway-api-userinfo'] ??
                this.req.headers['x-endpoint-api-userinfo'];

            return JSON.parse(String(Buffer.from(String(userinfo), 'base64')));
        } catch (err) {
            return null;
        }
    }

    public static getIPAddress(): string {
        const forwarded = this.req?.headers['x-forwarded-for'];
        return forwarded ? String(forwarded) : this.req?.socket?.remoteAddress ?? 'Ip Not Detected';
    }
}
