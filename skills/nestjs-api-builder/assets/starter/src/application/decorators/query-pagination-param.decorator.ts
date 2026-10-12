import { createParamDecorator, ExecutionContext, PipeTransform, Type } from '@nestjs/common';
import { plainToClass } from 'class-transformer';
import { FastifyRequest } from 'fastify';
import { QueryData, QueryRequest } from 'src/delivery/interfaces/query-request.interface';
import { DEFAULT_NUMBER_OF_SIZE, FIRST_PAGE } from 'src/utils/constants/general.constant';
import { filterExposeExclude } from 'src/utils/filter.util';
import { sortTransform } from 'src/utils/sort.util';
import { transformAndSerialize } from 'src/utils/transform.util';

export const getQuery = <T>(data: QueryData<T>, ctx: ExecutionContext): QueryRequest => {
    const { classConstructor, option } = data;
    const req: FastifyRequest = ctx.switchToHttp().getRequest();
    const query = (req.query ?? {}) as Record<string, any>;
    const { page, size } = query;
    const obj = plainToClass(classConstructor, {}, { excludeExtraneousValues: true });
    const keys = Object.keys(obj);
    const fixPage: number = !isNaN(Number(page)) && Number(page) > 0 ? Number(page) : FIRST_PAGE;
    const fixSize: number = !isNaN(Number(size)) && Number(size) > 0 ? Number(size) : DEFAULT_NUMBER_OF_SIZE;

    return new QueryRequest({
        page: fixPage,
        size: fixSize,
        sort: sortTransform(keys, query.sort),
        filter: transformAndSerialize<T>(classConstructor, keys, query),
        exclude: filterExposeExclude(query.exclude, keys, option?.defaultExclude),
        expose: filterExposeExclude(query.expose, keys, option?.defaultExpose)
    });
};

export const QueryFilterPagination: <T>(
    data: QueryData<T>,
    ...pipes: Array<Type<PipeTransform> | PipeTransform>
) => ParameterDecorator = createParamDecorator(getQuery);
