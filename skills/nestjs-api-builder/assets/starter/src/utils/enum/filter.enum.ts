// Query-string filter operators: ?<field>.<criteria>=<value>
// The "greather" spelling is part of the public API contract; keep it.
export enum FilterCriteria {
    equal = 'equal',
    notEqual = 'notEqual',
    greatherThanOrEqual = 'greatherThanOrEqual',
    lessThanOrEqual = 'lessThanOrEqual',
    greatherThan = 'greatherThan',
    lessThan = 'lessThan',
    includes = 'includes',
    in = 'in',
    contains = 'contains',
    between = 'between'
}
