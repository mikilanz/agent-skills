import { ArgumentMetadata, Injectable, ValidationPipe, ValidationPipeOptions } from '@nestjs/common';

@Injectable()
export class GeneralSanitizationPipe extends ValidationPipe {
    constructor(options?: ValidationPipeOptions) {
        super(options);
    }

    transform(value: any, metadata: ArgumentMetadata) {
        return commonSanitize(value);
    }
}

// Applies every sanitization in sequence to all strings, recursing into arrays and objects.
const commonSanitize = applySanitizations([trimString]);

function applySanitizations(sanitizations: Array<(value: string) => string>) {
    return function sanitize(value: any): any {
        switch (typeof value) {
            case 'string':
                return sanitizations.reduce((acc, sanitizeFn) => sanitizeFn(acc), value);
            case 'object':
                if (Array.isArray(value)) {
                    return value.map((item) => sanitize(item));
                } else if (value !== null) {
                    Object.keys(value).forEach((key) => {
                        value[key] = sanitize(value[key]);
                    });
                    return value;
                }
                break;
        }
        return value;
    };
}

function trimString(value: string): string {
    return value.trim();
}
