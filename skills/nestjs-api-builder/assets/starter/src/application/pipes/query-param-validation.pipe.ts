import { ArgumentMetadata, Injectable, ValidationPipe } from '@nestjs/common';
import { validate } from 'class-validator';

@Injectable()
export class QueryParamValidationPipe extends ValidationPipe {
    async transform(data: any, metadata: ArgumentMetadata) {
        const typeData: any = metadata.data;
        const errors: any = await validate(new typeData.classConstructor(data), this.validatorOptions);
        if (errors.length > 0) {
            throw this.exceptionFactory(errors);
        }
        Object.keys(data).forEach(key => {
            if (data[key] === null || data[key] === undefined) {
                delete data[key];
            }
        });
        return data;
    }
}
