import { CanActivate, ExecutionContext, Injectable } from '@nestjs/common';
import { Observable } from 'rxjs';

// The API gateway verifies the JWT and forwards the claims as a base64 JSON header.
@Injectable()
export class AuthGuard implements CanActivate {
    canActivate(
        context: ExecutionContext,
    ): boolean | Promise<boolean> | Observable<boolean> {
        const request = context.switchToHttp().getRequest();
        const endpointInfo = request.headers['x-endpoint-api-userinfo'];
        const apigatewayInfo = request.headers['x-apigateway-api-userinfo'];
        const userinfo = endpointInfo ? endpointInfo : apigatewayInfo ? apigatewayInfo : null;
        return userinfo !== null && userinfo !== undefined;
    }
}
