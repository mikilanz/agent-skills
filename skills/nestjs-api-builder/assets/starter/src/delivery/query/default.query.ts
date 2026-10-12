import { Expose } from 'class-transformer';

export class DefaultQuery {
    @Expose()
    id: string;

    @Expose()
    uuid: string;

    @Expose()
    createdAt: string;

    @Expose()
    updatedAt: string;
}
