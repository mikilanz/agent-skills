export function filterExposeExclude(
    data: any,
    classKeys: string[],
    defaultValue: string[] = []
): string[] {
    const filterValue = defaultValue.filter(key => classKeys.includes(key));

    if (!data) {
        return filterValue;
    }

    const values: string[] = Array.isArray(data) ? data : [data];

    return values.reduce((val, curr) => {
        curr.split(',').filter(key => classKeys.includes(key)).forEach(key => val.push(key));
        return val;
    }, filterValue);
}

export function valueOf(data: any, key: string): any {
    return key.split('.').reduce((prev, curr) => {
        return prev !== undefined && prev !== null && prev[curr] !== undefined && prev[curr] !== null
            ? prev[curr]
            : undefined;
    }, data);
}

// 'a.b.c', 1 -> { a: { b: { c: 1 } } }
export function stringToObject(key: string, value: any): Record<string, any> {
    return key.split('.').reduceRight((acc, curr) => ({ [curr]: acc }), value);
}
