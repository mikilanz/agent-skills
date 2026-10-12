import { Between, FindManyOptions, In, LessThan, LessThanOrEqual, Like, MoreThan, MoreThanOrEqual, Not } from 'typeorm';
import { QueryFilter, QueryRequest, SortData } from 'src/delivery/interfaces/query-request.interface';
import { FilterCriteria } from './enum/filter.enum';
import { stringToObject } from './filter.util';

function toOperator(filter: string, value: string | string[]) {
    switch (filter) {
        case FilterCriteria.notEqual:
            return Not(value);
        case FilterCriteria.greatherThanOrEqual:
            return MoreThanOrEqual(value);
        case FilterCriteria.lessThanOrEqual:
            return LessThanOrEqual(value);
        case FilterCriteria.greatherThan:
            return MoreThan(value);
        case FilterCriteria.lessThan:
            return LessThan(value);
        case FilterCriteria.includes:
        case FilterCriteria.contains:
            return Like(`%${value}%`);
        case FilterCriteria.in:
            return In(Array.isArray(value) ? value : value.split(','));
        case FilterCriteria.between: {
            const [from, to] = value.toString().split(',');
            return Between(new Date(from), new Date(to));
        }
        default:
            return value;
    }
}

function deepMerge(target: Record<string, any>, source: Record<string, any>) {
    Object.keys(source).forEach(key => {
        const value = source[key];
        const isPlainObject = value && typeof value === 'object' && value.constructor === Object;
        target[key] = isPlainObject ? deepMerge(target[key] ?? {}, value) : value;
    });
    return target;
}

// Nested keys ('account.companyCode') become nested where objects for relation filtering.
export function filterDataORM(queries: QueryFilter[]) {
    const where = queries.reduce((acc, { key, filter, value }) => {
        return deepMerge(acc, stringToObject(key, toOperator(filter, value)));
    }, {});

    return { where };
}

function typeORMOrder(sorts: SortData[]) {
    return sorts.reduce((acc, { key, order }) => {
        return deepMerge(acc, stringToObject(key, order.toUpperCase()));
    }, {});
}

export function typeORMQuery<T = any>(query: QueryRequest): FindManyOptions<T> {
    const pagination = query.pagination ? {
        skip: query.size * query.page,
        take: query.size
    } : {};

    return {
        ...filterDataORM(query.filter),
        order: typeORMOrder(query.sort),
        ...pagination
    };
}
