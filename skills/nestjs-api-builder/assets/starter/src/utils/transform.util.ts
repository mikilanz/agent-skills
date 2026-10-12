import { ClassConstructor, plainToClass } from 'class-transformer';
import { QueryFilter } from 'src/delivery/interfaces/query-request.interface';
import { FilterCriteria } from './enum/filter.enum';
import { stringToObject, valueOf } from './filter.util';

// Turns ?<field>.<criteria>=<value> query params into QueryFilter[], keeping only
// fields @Expose()d on the query class and known FilterCriteria.
export function transformAndSerialize<T>(cls: ClassConstructor<T>, classKeys: string[], data: Record<string, any>): QueryFilter[] {
    return Object.keys(data).reduce((val, curr) => {
        if (!curr.includes('.')) {
            return val;
        }

        const filter = curr.substring(curr.lastIndexOf('.') + 1);
        const key = curr.substring(0, curr.lastIndexOf('.'));
        const parent = key.split('.')[0];

        if (classKeys.includes(parent) && (Object.values(FilterCriteria) as string[]).includes(filter)) {
            const transform = JSON.parse(
                JSON.stringify(
                    plainToClass(cls, stringToObject(key, data[curr]), { excludeExtraneousValues: true })
                )
            );

            val.push({
                key,
                filter,
                value: valueOf(transform, key)
            });
        }

        return val;
    }, new Array<QueryFilter>());
}

// Field-level diff used by audit subscribers: returns keys whose value differs.
export function getDifferenceObject(oldObject: object, newObject: object): Record<string, any> {
    if (!newObject || Object.prototype.toString.call(newObject) !== '[object Object]') {
        return { ...oldObject };
    }

    const keys = new Set([...Object.keys(oldObject ?? {}), ...Object.keys(newObject)]);
    return Array.from(keys).reduce((diffs, key) => {
        if (JSON.stringify(oldObject?.[key]) !== JSON.stringify(newObject[key])) {
            diffs[key] = newObject[key] === undefined ? null : newObject[key];
        }
        return diffs;
    }, {});
}
