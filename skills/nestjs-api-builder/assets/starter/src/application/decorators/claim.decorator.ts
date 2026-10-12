import { createParamDecorator, ExecutionContext, UnauthorizedException } from '@nestjs/common';
import { Claim } from 'src/delivery/dto/claim.dto';

export const ClaimInfo = createParamDecorator(
    (data: string, ctx: ExecutionContext) => {
        try {
            const request = ctx.switchToHttp().getRequest();
            const endpointInfo = request.headers['x-endpoint-api-userinfo'];
            const apigatewayInfo = request.headers['x-apigateway-api-userinfo'];
            const userInfo = endpointInfo ? endpointInfo : apigatewayInfo;
            const user: Claim = JSON.parse(Buffer.from(userInfo, 'base64').toString());

            return data ? user && user[data] : user;
        } catch (err) {
            throw new UnauthorizedException();
        }
    },
);
