import { ApiProperty } from '@nestjs/swagger';

// Returned by list services; HeadersInterceptor moves headers onto the reply and returns data.
export class CustomResponseDTO<T> {
    @ApiProperty({ description: 'List of items', isArray: true })
    data: T;

    @ApiProperty({ type: 'object', example: { 'x-total-count': '100' }, description: 'Response headers, e.g. x-total-count.' })
    headers: Record<string, string>;
}
