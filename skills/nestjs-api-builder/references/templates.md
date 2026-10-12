# Templates

A complete example feature, `Widget`, with CRUD, a paginated list, tenant scoping, an audit subscriber and an
async external sync through CQRS. Every file here type-checks against the starter in `assets/starter/`,
and the spec passes. Rename `Widget` / `widget` / `widgets` to the real resource.

When you add to an existing project, match **that** project's quotes, indentation and line endings, and
its real names for shared pieces (role enum, claim decorator, base entity). The structure stays the same.

## 1. Migration

`<ts>` must be greater than the largest existing migration prefix (`ls src/migrations | sort | tail -1`).
Export constraint names so the entity and service can reuse them.

`src/migrations/1700000000100-AddWidget.ts`

```ts
import { Logger } from '@nestjs/common';
import { configService } from 'src/config/config.service';
import { MigrationInterface, QueryRunner } from 'typeorm';

export const widgetNameUnique = 'widgets_name_unique';

export class AddWidget1700000000100 implements MigrationInterface {
    protected readonly logger = new Logger(AddWidget1700000000100.name);
    protected readonly configService = configService;

    public async up(queryRunner: QueryRunner): Promise<void> {
        await queryRunner.query(`CREATE TABLE IF NOT EXISTS ${this.configService.get('DB_SCHEMA')}.widgets (
            "id" SERIAL4 NOT NULL,
            "uuid" VARCHAR(255) NULL DEFAULT uuid_generate_v4(),
            "companyCode" VARCHAR(255) NOT NULL,
            "name" VARCHAR(255) NOT NULL,
            "type" VARCHAR(50) NOT NULL,
            "enabled" BOOLEAN NOT NULL DEFAULT false,
            "externalId" VARCHAR(255) NULL,
            "notes" TEXT NULL,
            "createdAt" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            "updatedAt" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT widgets_pk PRIMARY KEY ("id"),
            CONSTRAINT ${widgetNameUnique} UNIQUE ("name")
        )`);
        await queryRunner.query(`CREATE INDEX IF NOT EXISTS widgets_companycode_idx ON ${this.configService.get('DB_SCHEMA')}.widgets ("companyCode")`);
    }

    public async down(queryRunner: QueryRunner): Promise<void> {
        this.logger.warn("To execute down please run 'DROP TABLE \"widgets\"'");
    }
}
```

Alter-column migration (reversible `down`):

```ts
public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`ALTER TABLE ${this.configService.get('DB_SCHEMA')}.widgets ADD COLUMN IF NOT EXISTS "color" VARCHAR(20) NULL`);
}

public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`ALTER TABLE ${this.configService.get('DB_SCHEMA')}.widgets DROP COLUMN IF EXISTS "color"`);
}
```

Foreign key: add `"accountId" INT NOT NULL` and `CONSTRAINT widgets_account_fk FOREIGN KEY ("accountId") REFERENCES ${schema}.accounts ("id")`.

## 2. Enum

`src/delivery/enums/widget.enum.ts`: add it to `delivery/enums/index.ts` too

```ts
export enum WidgetType {
    BASIC = 'basic',
    PREMIUM = 'premium',
}
```

## 3. Entity

`src/data-provider/entities/widget.entity.ts`

```ts
import { Column, Entity, Unique } from 'typeorm';
import { DefaultEntity } from './default.entity';
import { WidgetType } from 'src/delivery/enums/widget.enum';
import { widgetNameUnique } from 'src/migrations/1700000000100-AddWidget';

@Entity({ name: 'widgets' })
@Unique(widgetNameUnique, ['name'])
export class WidgetEntity extends DefaultEntity {
    @Column({ length: 255 })
    companyCode: string;

    @Column({ length: 255 })
    name: string;

    @Column({ length: 50 })
    type: WidgetType;

    @Column({ type: 'boolean', default: false })
    enabled: boolean;

    @Column({ length: 255, nullable: true })
    externalId: string | null;

    @Column({ type: 'text', nullable: true })
    notes: string | null;
}
```

Register it:
- `data-provider/entities/index.ts`: `import { WidgetEntity } from './widget.entity';` and add it to `export { … }`.
- `application/modules/typeorm.module.ts`: add it to the `entities` list, which feeds both `forRootAsync` and `forFeature`. In projects with two separate lists, add it to **both**.

Relation example (explicit FK column + relation property):

```ts
@Column()
accountId: number;

@ManyToOne(() => AccountEntity, account => account.widgets)
@JoinColumn({ name: 'accountId' })
account: AccountEntity;

// on AccountEntity
@OneToMany(() => WidgetEntity, widget => widget.account)
widgets: WidgetEntity[];
```

Computed, non-column field: declare the property and fill it in an `@AfterLoad()` method.

## 4. DTOs

`src/delivery/dto/widget/widget.dto.ts`

```ts
import { ApiProperty } from '@nestjs/swagger';
import { Transform } from 'class-transformer';
import { IsArray, IsBoolean, IsIn, IsNotEmpty, IsOptional, IsString, IsUUID } from 'class-validator';
import { WidgetType } from 'src/delivery/enums/widget.enum';
import { v4 } from 'uuid';

export class CreateWidgetDTO {
    @ApiProperty()
    @IsNotEmpty()
    @IsString()
    @Transform(({ value }) => value?.toLowerCase())
    name: string;

    @ApiProperty({ enum: Object.values(WidgetType) })
    @IsIn(Object.values(WidgetType))
    type: WidgetType;

    @ApiProperty({ required: false, default: false })
    @IsBoolean()
    @IsOptional()
    enabled?: boolean;

    @ApiProperty({ required: false })
    @IsString()
    @IsOptional()
    notes?: string;

    // set by the controller from the claim, never by the client
    companyCode?: string;
    actionPerformedBy?: string;
}

export class UpdateWidgetDTO {
    @ApiProperty({ example: v4() })
    @IsUUID()
    uuid: string;

    @ApiProperty({ required: false })
    @IsString()
    @IsOptional()
    name?: string;

    @ApiProperty({ required: false })
    @IsBoolean()
    @IsOptional()
    enabled?: boolean;

    @ApiProperty({ required: false })
    @IsString()
    @IsOptional()
    notes?: string;
}

export class DeleteWidgetDTO {
    @ApiProperty({ example: v4() })
    @IsUUID()
    uuid: string;
}

export class BulkDeleteWidgetDTO {
    @ApiProperty({ example: [v4()] })
    @IsArray()
    @IsUUID('all', { each: true })
    uuids: string[];
}
```

- Response DTOs built in code get `constructor(partial: Partial<X>) { Object.assign(this, partial); }`.
- Nested arrays need `@ValidateNested({ each: true }) @Type(() => ChildDTO)` plus `@ApiProperty({ type: [ChildDTO] })`.
- Several groups in one DTO file are separated with a `/////` banner comment.

## 5. Query class (filter and sort whitelist)

`src/delivery/query/widget.query.ts`

```ts
import { Expose } from 'class-transformer';
import { DefaultQuery } from './default.query';

export class WidgetQuery extends DefaultQuery {
    @Expose()
    companyCode: string;

    @Expose()
    name: string;

    @Expose()
    type: string;

    @Expose()
    enabled: boolean;

    constructor(partial: Partial<WidgetQuery>) {
        super();
        Object.assign(this, partial);
    }
}
```

Only `@Expose()`d fields can be filtered or sorted. Nested relation filters use the relation name as the first
key segment (`?account.companyCode.equal=abc`), so expose `account` here.

## 6. Service

`src/use-case/widget/widget.service.ts`: export it from `use-case/index.ts`

```ts
import { BadRequestException, HttpStatus, Injectable, Logger, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { CommandBus } from '@nestjs/cqrs';
import { In, Repository } from 'typeorm';
import { from, Observable, throwError } from 'rxjs';
import { catchError, map, switchMap, tap } from 'rxjs/operators';
import { WidgetEntity } from 'src/data-provider/entities';
import { CreateWidgetDTO, DeleteWidgetDTO, UpdateWidgetDTO } from 'src/delivery/dto/widget/widget.dto';
import { CustomResponseDTO } from 'src/delivery/dto/custom-response.dto';
import { DefaultHttpResponseDTO } from 'src/delivery/dto/default.dto';
import { QueryRequest } from 'src/delivery/interfaces/query-request.interface';
import { typeORMQuery } from 'src/utils/typeorm.util';
import { widgetNameUnique } from 'src/migrations/1700000000100-AddWidget';
import { SyncWidgetExternalCommand } from 'src/use-case/cqrs/commands/actions/external/sync-widget-external.command';

@Injectable()
export class WidgetService {
    private readonly logger: Logger = new Logger(WidgetService.name);

    constructor(
        @InjectRepository(WidgetEntity)
        private readonly widgetRepository: Repository<WidgetEntity>,
        private readonly commandBus: CommandBus,
    ) { }

    create(dto: CreateWidgetDTO): Observable<WidgetEntity> {
        const widget = this.widgetRepository.create(dto);
        return from(this.widgetRepository.save(widget)).pipe(
            // fire-and-forget: the response does not wait for the external sync
            tap((saved) => this.commandBus.execute(new SyncWidgetExternalCommand(saved.companyCode, saved.id, saved.name, dto.actionPerformedBy))),
            catchError((err) => {
                // 23505 = Postgres unique_violation
                if (err?.code === '23505' && err?.constraint?.includes(widgetNameUnique)) {
                    return throwError(() => new BadRequestException(`Duplicate entry: The '${dto.name}' widget already exists`));
                }
                return throwError(() => err);
            })
        );
    }

    findAll(query: QueryRequest, companyCode?: string): Observable<CustomResponseDTO<WidgetEntity[]>> {
        const options = typeORMQuery<WidgetEntity>(query);
        if (companyCode) {
            options.where = { ...options.where, companyCode };
        }

        return from(this.widgetRepository.findAndCount(options)).pipe(
            map(([data, count]) => ({
                data,
                headers: { 'x-total-count': count.toString() }
            })),
            catchError((err) => throwError(() => err))
        );
    }

    findOne(uuid: string): Observable<WidgetEntity> {
        return from(this.widgetRepository.findOne({ where: { uuid } })).pipe(
            map((widget) => {
                if (!widget) {
                    throw new NotFoundException(`Widget with UUID "${uuid}" not found`);
                }
                return widget;
            }),
            catchError((err) => throwError(() => err))
        );
    }

    update(dto: UpdateWidgetDTO): Observable<WidgetEntity> {
        return from(this.widgetRepository.findOne({ where: { uuid: dto.uuid } })).pipe(
            switchMap((widget) => {
                if (!widget) {
                    throw new NotFoundException(`Widget with UUID "${dto.uuid}" not found`);
                }
                Object.assign(widget, dto);
                return from(this.widgetRepository.save(widget));
            }),
            catchError((err) => throwError(() => err))
        );
    }

    remove(dto: DeleteWidgetDTO): Observable<DefaultHttpResponseDTO> {
        return from(this.widgetRepository.findOne({ where: { uuid: dto.uuid } })).pipe(
            switchMap((widget) => {
                if (!widget) {
                    throw new NotFoundException(`Widget with UUID "${dto.uuid}" not found`);
                }
                // remove() rather than delete() so subscribers get afterRemove with the entity
                return from(this.widgetRepository.remove(widget));
            }),
            map(() => new DefaultHttpResponseDTO({
                statusCode: HttpStatus.OK,
                messages: 'success',
                ok: true
            })),
            catchError((err) => throwError(() => err))
        );
    }

    removeBulk(uuids: string[]): Observable<DefaultHttpResponseDTO> {
        return from(this.widgetRepository.delete({ uuid: In(uuids) })).pipe(
            map((result) => new DefaultHttpResponseDTO({
                statusCode: HttpStatus.OK,
                messages: `${result.affected} widget(s) deleted`,
                ok: true
            })),
            catchError((err) => throwError(() => err))
        );
    }
}
```

Wrapping unexpected failures in reporting-style services:

```ts
catchError((err) => {
    this.logger.error(`Failed to get widget report: ${err.message}`, err.stack);
    return throwError(() => new HttpException('Failed to retrieve widget report.', HttpStatus.INTERNAL_SERVER_ERROR));
})
```

Aggregates use the query builder:

```ts
const rows$ = from(this.widgetRepository.createQueryBuilder('widget')
    .select('widget.companyCode', 'companyCode')
    .addSelect('COUNT(widget.id)', 'total')
    .where('widget.createdAt >= :startDate', { startDate })
    .groupBy('widget.companyCode')
    .getRawMany<{ companyCode: string, total: string }>());
```

Parallel reads use `forkJoin([a$, b$]).pipe(map(([a, b]) => …))`.

## 7. Controller

`src/delivery/http/widget.controller.ts`: export it from `delivery/http/index.ts`

```ts
import { Body, Controller, Delete, Get, HttpStatus, Param, Post, Put, UseGuards, UseInterceptors } from '@nestjs/common';
import { ApiHeader, ApiOperation, ApiParam, ApiResponse, ApiTags } from '@nestjs/swagger';
import { Observable } from 'rxjs';
import { AuthGuard } from 'src/application/guards/auth.guard';
import { HeadersInterceptor } from 'src/application/interceptors/headers.interceptor';
import { ClaimInfo } from 'src/application/decorators/claim.decorator';
import { QueryFilterPagination } from 'src/application/decorators/query-pagination-param.decorator';
import { ClaimRoleValidationPipe } from 'src/application/pipes/claim-role-validation.pipe';
import { QueryParamValidationPipe } from 'src/application/pipes/query-param-validation.pipe';
import { WidgetEntity } from 'src/data-provider/entities';
import { WidgetService } from 'src/use-case';
import { Claim, ClaimRole } from '../dto/claim.dto';
import { CustomResponseDTO } from '../dto/custom-response.dto';
import { DefaultHttpResponseDTO } from '../dto/default.dto';
import { BulkDeleteWidgetDTO, CreateWidgetDTO, DeleteWidgetDTO, UpdateWidgetDTO } from '../dto/widget/widget.dto';
import { QueryRequest } from '../interfaces/query-request.interface';
import { WidgetQuery } from '../query/widget.query';

@UseGuards(new AuthGuard())
@ApiTags('widget')
@Controller('widget')
export class WidgetController {
    constructor(private readonly widgetService: WidgetService) { }

    @Post(':companyCode')
    @ApiHeader({ name: 'x-apigateway-api-userinfo', required: true })
    @ApiOperation({ summary: 'Create widget' })
    @ApiParam({ name: 'companyCode', type: 'string', required: true, description: 'Company code (only honoured for superadmin)' })
    @ApiResponse({ status: HttpStatus.CREATED, type: WidgetEntity, description: 'The record has been successfully created.' })
    create(
        @ClaimInfo(new ClaimRoleValidationPipe([ClaimRole.SUPERADMIN, ClaimRole.ADMIN, ClaimRole.EMPLOYEE])) claim: Claim,
        @Param('companyCode') companyCode: string,
        @Body() body: CreateWidgetDTO
    ): Observable<WidgetEntity> {
        body.companyCode = claim?.role === ClaimRole.SUPERADMIN ? companyCode : claim?.companyCode;
        body.actionPerformedBy = claim?.username;
        return this.widgetService.create(body);
    }

    @Get('/all')
    @ApiHeader({ name: 'x-apigateway-api-userinfo', required: true })
    @ApiOperation({ summary: 'Get all widgets' })
    @ApiResponse({ status: HttpStatus.OK, type: [WidgetEntity], description: 'List of widgets. Total count in the x-total-count header.' })
    @ApiResponse({ status: HttpStatus.UNAUTHORIZED, description: 'Unauthorized.' })
    @UseInterceptors(HeadersInterceptor)
    findAll(
        @ClaimInfo(new ClaimRoleValidationPipe([ClaimRole.SUPERADMIN, ClaimRole.ADMIN, ClaimRole.EMPLOYEE])) claim: Claim,
        @QueryFilterPagination<WidgetQuery>(
            {
                classConstructor: WidgetQuery,
            },
            new QueryParamValidationPipe()
        ) query: QueryRequest
    ): Observable<CustomResponseDTO<WidgetEntity[]>> {
        const companyCode = claim?.role === ClaimRole.SUPERADMIN ? undefined : claim?.companyCode;
        return this.widgetService.findAll(query, companyCode);
    }

    @Get('/single/:uuid')
    @ApiHeader({ name: 'x-apigateway-api-userinfo', required: true })
    @ApiOperation({ summary: 'Get a single widget' })
    @ApiParam({ name: 'uuid', type: 'string', required: true, description: 'Widget UUID' })
    @ApiResponse({ status: HttpStatus.OK, type: WidgetEntity, description: 'Details of the widget.' })
    @ApiResponse({ status: HttpStatus.NOT_FOUND, description: 'Widget not found.' })
    findOne(
        @ClaimInfo(new ClaimRoleValidationPipe([ClaimRole.SUPERADMIN])) _: Claim,
        @Param('uuid') uuid: string
    ): Observable<WidgetEntity> {
        return this.widgetService.findOne(uuid);
    }

    @Put('')
    @ApiHeader({ name: 'x-apigateway-api-userinfo', required: true })
    @ApiOperation({ summary: 'Update a widget' })
    @ApiResponse({ status: HttpStatus.OK, type: WidgetEntity, description: 'The record has been successfully updated.' })
    @ApiResponse({ status: HttpStatus.NOT_FOUND, description: 'Widget not found.' })
    update(
        @ClaimInfo(new ClaimRoleValidationPipe([ClaimRole.SUPERADMIN])) _: Claim,
        @Body() body: UpdateWidgetDTO
    ): Observable<WidgetEntity> {
        return this.widgetService.update(body);
    }

    @Delete('')
    @ApiHeader({ name: 'x-apigateway-api-userinfo', required: true })
    @ApiOperation({ summary: 'Delete a widget' })
    @ApiResponse({ status: HttpStatus.OK, type: DefaultHttpResponseDTO, description: 'The record has been successfully deleted.' })
    @ApiResponse({ status: HttpStatus.NOT_FOUND, description: 'Widget not found.' })
    remove(
        @ClaimInfo(new ClaimRoleValidationPipe([ClaimRole.SUPERADMIN])) _: Claim,
        @Body() body: DeleteWidgetDTO
    ): Observable<DefaultHttpResponseDTO> {
        return this.widgetService.remove(body);
    }

    @Delete('/bulk')
    @ApiHeader({ name: 'x-apigateway-api-userinfo', required: true })
    @ApiOperation({ summary: 'Bulk delete widgets' })
    @ApiResponse({ status: HttpStatus.OK, type: DefaultHttpResponseDTO, description: 'The records have been successfully deleted.' })
    removeBulk(
        @ClaimInfo(new ClaimRoleValidationPipe([ClaimRole.SUPERADMIN])) _: Claim,
        @Body() body: BulkDeleteWidgetDTO
    ): Observable<DefaultHttpResponseDTO> {
        return this.widgetService.removeBulk(body.uuids);
    }
}
```

## 8. Module

`src/application/modules/widget.module.ts`

```ts
import { Module } from '@nestjs/common';
import { WidgetController } from 'src/delivery/http';
import { WidgetService } from 'src/use-case';
import { WidgetSubscriber } from 'src/subscribers/_index';

@Module({
    controllers: [WidgetController],
    providers: [
        WidgetService,
        WidgetSubscriber
    ],
    exports: [WidgetService]
})
export class WidgetModule { }
```

Then add it to `application/modules/index.ts` and to `AppModule.imports`. Repositories need no
`forFeature` here, because the global TypeOrm module already exports them.

## 9. Audit subscriber

`src/subscribers/widget.subscriber.ts`: export it from `subscribers/_index.ts` and provide it in the feature module

```ts
import { DataSource, EntitySubscriberInterface, EventSubscriber, InsertEvent, RemoveEvent, UpdateEvent } from 'typeorm';
import { InjectDataSource } from '@nestjs/typeorm';
import { Logger } from '@nestjs/common';
import { Context, WidgetEntity } from 'src/data-provider/entities';
import { getDifferenceObject } from 'src/utils/transform.util';

// Audit trail for widgets. Replace the logger calls with the project's audit sink
// (activity-log API, audit table, Kafka topic) when one exists.
@EventSubscriber()
export class WidgetSubscriber implements EntitySubscriberInterface<WidgetEntity> {
    private readonly logger: Logger = new Logger(WidgetSubscriber.name);

    constructor(@InjectDataSource() readonly dataSource: DataSource) {
        dataSource.subscribers.push(this);
    }

    listenTo() {
        return WidgetEntity;
    }

    afterInsert(event: InsertEvent<WidgetEntity>) {
        this.audit('CREATE', event.entity);
    }

    afterUpdate(event: UpdateEvent<WidgetEntity>) {
        // databaseEntity is the row before the update
        const changes = getDifferenceObject(event.databaseEntity ?? {}, event.entity ?? {});
        this.audit('UPDATE', event.entity, changes);
    }

    afterRemove(event: RemoveEvent<WidgetEntity>) {
        this.audit('DELETE', event.databaseEntity ?? event.entity);
    }

    private audit(action: string, value: any, changes?: any) {
        const user = Context.getUser();
        this.logger.log(JSON.stringify({ action, entity: 'widgets', by: user?.username, ip: Context.getIPAddress(), value, changes }));
    }
}
```

Subscribers fire on `save()` / `remove()`, not on `update()` / `delete()` query shortcuts.

## 10. CQRS: async side effect

Flow: the service calls `commandBus.execute(Command)` → CommandHandler → AggregateRoot model `apply(Event)` →
EventsHandler does the slow or external work and subscribes. The HTTP response doesn't wait.

`src/use-case/cqrs/commands/actions/external/sync-widget-external.command.ts`

```ts
export class SyncWidgetExternalCommand {
    constructor(
        public readonly companyCode: string,
        public readonly widgetId: number,
        public readonly name: string,
        public readonly actionPerformedBy: string
    ) { }
}
```

`src/use-case/cqrs/events/actions/external/sync-widget-external.event.ts`

```ts
export class SyncWidgetExternalEvent {
    constructor(
        public readonly companyCode: string,
        public readonly widgetId: number,
        public readonly name: string,
        public readonly actionPerformedBy: string
    ) { }
}
```

`src/use-case/cqrs/models/widget.model.ts`

```ts
import { AggregateRoot } from '@nestjs/cqrs';
import { SyncWidgetExternalEvent } from '../events/actions/external/sync-widget-external.event';

export class WidgetModel extends AggregateRoot {
    constructor(
        private companyCode: string,
        private widgetId: number,
        private actionPerformedBy: string
    ) {
        super();
        this.autoCommit = true;
    }

    syncWidgetExternal(name: string) {
        this.apply(new SyncWidgetExternalEvent(this.companyCode, this.widgetId, name, this.actionPerformedBy));
    }
}
```

`src/use-case/cqrs/commands/handlers/external/sync-widget-external.handler.ts`

```ts
import { CommandHandler, EventPublisher, ICommandHandler } from '@nestjs/cqrs';
import { SyncWidgetExternalCommand } from '../../actions/external/sync-widget-external.command';
import { WidgetModel } from '../../../models';

@CommandHandler(SyncWidgetExternalCommand)
export class SyncWidgetExternalCommandHandler implements ICommandHandler<SyncWidgetExternalCommand> {
    constructor(private publisher: EventPublisher) { }

    async execute(command: SyncWidgetExternalCommand) {
        const { companyCode, widgetId, name, actionPerformedBy } = command;
        const widget = this.publisher.mergeObjectContext(new WidgetModel(companyCode, widgetId, actionPerformedBy));

        widget.syncWidgetExternal(name);
    }
}
```

`src/use-case/cqrs/events/handlers/external/sync-widget-external.handler.ts`

```ts
import { CommandBus, EventsHandler, IEventHandler } from '@nestjs/cqrs';
import { Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { HttpService } from '@nestjs/axios';
import { Repository } from 'typeorm';
import { catchError, from, map, of, retry, switchMap, tap } from 'rxjs';
import { ConfigService } from 'src/config';
import { WidgetEntity } from 'src/data-provider/entities';
import { SendKafkaMessageCommand } from 'src/use-case/cqrs/commands/actions/send-kafka-message.command';
import { SyncWidgetExternalEvent } from '../../actions/external/sync-widget-external.event';

@EventsHandler(SyncWidgetExternalEvent)
export class SyncWidgetExternalEventHandler implements IEventHandler<SyncWidgetExternalEvent> {
    private readonly logger: Logger = new Logger(SyncWidgetExternalEventHandler.name);

    constructor(
        @InjectRepository(WidgetEntity)
        private readonly widgetRepository: Repository<WidgetEntity>,
        private readonly httpService: HttpService,
        private readonly configService: ConfigService,
        private readonly commandBus: CommandBus,
    ) { }

    handle(event: SyncWidgetExternalEvent) {
        const { companyCode, widgetId, name, actionPerformedBy } = event;
        const kafkaTopic = this.configService.get('NODE_ENV') == 'production' ? 'widget_action_prod' : 'widget_action_stage';

        this.httpService.post(`${this.configService.get('WIDGET_ENGINE_BASE_URL')}/widgets`, { companyCode, name }).pipe(
            retry({ count: 2, delay: 1000 }),
            map((res) => res.data),
            switchMap((data) => from(this.widgetRepository.update({ id: widgetId }, { externalId: data?.id }))),
            tap(() => this.commandBus.execute(new SendKafkaMessageCommand(kafkaTopic, { action: 'widget_synced', companyCode, widgetId, actionPerformedBy }))),
            catchError((err) => {
                this.logger.error(`(${companyCode}) Failed to sync widget ${widgetId}: ${err.message}`);
                return of(null);
            })
        ).subscribe();
    }
}
```

Register:

```ts
// use-case/cqrs/models/index.ts: add WidgetModel to the barrel

// use-case/cqrs/commands/index.ts
export const WidgetExternalCommandHandler = [
    SyncWidgetExternalCommand,
    SyncWidgetExternalCommandHandler
];

// use-case/cqrs/events/index.ts
export const WidgetExternalEventHandler = [
    SyncWidgetExternalEvent,
    SyncWidgetExternalEventHandler
];

// app.module.ts providers
...WidgetExternalCommandHandler,
...WidgetExternalEventHandler
```

Kafka from anywhere: `this.commandBus.execute(new SendKafkaMessageCommand(topic, payload))`.

## 11. Outbound HTTP client method

```ts
getWidgetFromEngine(code: string): Observable<WidgetEngineResponse> {
    const url = `${this.configService.get('WIDGET_ENGINE_BASE_URL')}/widgets/${code}`;

    return from(this.authClient.getAuthToken()).pipe(
        switchMap(token => this.httpService.get(url, {
            headers: { 'authorization': `Bearer ${token.id_token}` },
        })),
        map((res) => res.data),
        catchError((err) => {
            const { response } = err;
            throw new HttpException(
                { statusCode: response?.status, message: response?.data, ok: false },
                response?.status ?? HttpStatus.INTERNAL_SERVER_ERROR,
            );
        })
    );
}
```

Put `WidgetEngineResponse` in `delivery/interfaces/widget.interface.ts`. A client used by many features
lives in its own `@Global()` module (see `CacheModule` / `KafkaModule` in the starter).

## 12. New env var

1. Joi schema in `config/custom-validation-function.ts`, in the matching `// === Section ===` group:
   `WIDGET_ENGINE_BASE_URL: Joi.string().required(),` or `WIDGET_TTL_SECOND: Joi.number().default(60), // in seconds`
2. `.env.example` under a matching `# === Section ===` header, and `docker-compose.yaml` if it lists env vars.
3. If several places use it, add a typed getter on `ConfigService`: `getWidgetEngineBaseURL(): string { return this.get('WIDGET_ENGINE_BASE_URL'); }`
4. Never put secrets, keys or webhook URLs as Joi defaults.

## 13. Custom class-validator decorator (`application/validators/`)

```ts
import { registerDecorator, ValidationArguments, ValidationOptions, ValidatorConstraint, ValidatorConstraintInterface } from 'class-validator';

@ValidatorConstraint({ async: true })
export class IsNotSubdomainConstraint implements ValidatorConstraintInterface {
    validate(value: any, args: ValidationArguments) {
        return typeof value === 'string' && value.split('.').length <= 2;
    }

    defaultMessage(args: ValidationArguments) {
        return `${args.property} must not be a subdomain`;
    }
}

export function IsNotSubdomain(validationOptions?: ValidationOptions) {
    return function (object: object, propertyName: string) {
        registerDecorator({ target: object.constructor, propertyName, options: validationOptions, constraints: [], validator: IsNotSubdomainConstraint });
    };
}
```

Constraints can inject providers (for DB lookups) because `main.ts` calls `useContainer(app.select(AppModule), …)`.
The constraint must then be `@Injectable()` and listed in a module's providers.

## 14. Custom error with special handling

```ts
// application/errors/widget-sync.error.ts
export class WidgetSyncError extends HttpException {
    constructor(message: string, status: HttpStatus = HttpStatus.BAD_REQUEST) {
        super(message, status);
    }
}
```

The starter's `ErrorsInterceptor` already formats every `HttpException`. Add an `instanceof WidgetSyncError`
branch in `handleError` only when the error needs extra behavior, such as alerting. Projects whose interceptor switches on
`err.constructor.name` need an explicit `case`, or the error body reports 500.

## 15. Unit test

`src/use-case/widget/widget.service.spec.ts`

```ts
import { Test, TestingModule } from '@nestjs/testing';
import { getRepositoryToken } from '@nestjs/typeorm';
import { CommandBus } from '@nestjs/cqrs';
import { NotFoundException } from '@nestjs/common';
import { lastValueFrom } from 'rxjs';
import { WidgetEntity } from 'src/data-provider/entities';
import { WidgetService } from './widget.service';

describe('WidgetService', () => {
    let service: WidgetService;
    const repository = {
        create: jest.fn(),
        save: jest.fn(),
        findOne: jest.fn(),
        findAndCount: jest.fn(),
        remove: jest.fn(),
        delete: jest.fn(),
    };

    beforeEach(async () => {
        jest.resetAllMocks();
        const module: TestingModule = await Test.createTestingModule({
            providers: [
                WidgetService,
                { provide: getRepositoryToken(WidgetEntity), useValue: repository },
                { provide: CommandBus, useValue: { execute: jest.fn() } },
            ],
        }).compile();

        service = module.get<WidgetService>(WidgetService);
    });

    it('throws NotFoundException when the widget does not exist', async () => {
        repository.findOne.mockResolvedValue(null);

        await expect(lastValueFrom(service.findOne('missing-uuid'))).rejects.toBeInstanceOf(NotFoundException);
    });

    it('returns data with x-total-count for findAll', async () => {
        repository.findAndCount.mockResolvedValue([[{ id: 1 }], 1]);

        const result = await lastValueFrom(service.findAll({ page: 0, size: 20, pagination: true, sort: [], filter: [], expose: [], exclude: [] }));

        expect(result.headers['x-total-count']).toBe('1');
        expect(result.data).toHaveLength(1);
    });
});
```
