import { ApiProperty } from '@nestjs/swagger';
import { HttpStatus } from '@nestjs/common';

export class DefaultHttpResponseDTO {
    @ApiProperty({ example: HttpStatus.OK, enum: HttpStatus, description: 'The HTTP status code of the response.' })
    statusCode: HttpStatus;

    @ApiProperty({ type: 'object', example: {}, required: false, description: 'The result of the request, an object or an array of objects.' })
    result?: any | any[];

    @ApiProperty({ type: 'object', example: {}, description: 'Errors that occurred during the request. Empty if none.' })
    errors?: any;

    @ApiProperty({ type: 'object', example: {}, description: 'Success messages or additional information.' })
    messages: any;

    @ApiProperty({ example: true, description: 'Whether the request was processed successfully.' })
    ok: boolean;

    constructor(partial: Partial<DefaultHttpResponseDTO>) {
        Object.assign(this, partial);
    }
}
