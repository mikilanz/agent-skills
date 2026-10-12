import { ClassConstructor } from 'class-transformer';
import { SortOrder } from '../enums/sort-order.enum';

export class QueryRequest {
    page: number;
    size: number;
    sort: SortData[];
    exclude: string[];
    expose: string[];
    pagination?: boolean = true;
    filter: QueryFilter[];

    constructor(partial: Partial<QueryRequest>) {
        Object.assign(this, partial);
    }
}

export interface SortData {
    key: string;
    order: SortOrder;
}

export interface QueryFilter {
    key: string;
    filter: string;
    value: string | string[];
}

export interface QueryOption {
    defaultExclude?: string[];
    defaultExpose?: string[];
}

export interface QueryData<T> {
    classConstructor: ClassConstructor<T>;
    option?: QueryOption;
}
