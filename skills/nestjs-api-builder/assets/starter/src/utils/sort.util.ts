import { BadRequestException } from '@nestjs/common';
import { SortData } from 'src/delivery/interfaces/query-request.interface';
import { SortOrder } from 'src/delivery/enums/sort-order.enum';

export function sortTransform(classKeys: string[], sort: any): SortData[] {
    if (!sort) {
        return [];
    }

    const values: string[] = Array.isArray(sort) ? sort : [sort];

    return values.reduce((sorts, val) => {
        if (!/^[A-Za-z0-9_.-]+,(desc|asc)$/.test(val.toString())) {
            throw new BadRequestException('Sort value should be [fieldName],[asc/desc]');
        }

        const [key, order] = val.split(',');
        if (classKeys.includes(key.split('.')[0])) {
            sorts.push({ key, order: order as SortOrder });
        }
        return sorts;
    }, new Array<SortData>());
}
