import { ArgumentMetadata, PipeTransform, UnauthorizedException } from '@nestjs/common';
import { ClaimRole } from 'src/delivery/dto/claim.dto';

export class ClaimRoleValidationPipe implements PipeTransform<any> {
    constructor(private readonly roles: ClaimRole[]) { }

    transform(data: any, metadata: ArgumentMetadata) {
        if (!this.roles.includes(data?.role)) {
            throw new UnauthorizedException();
        }
        return data;
    }
}
