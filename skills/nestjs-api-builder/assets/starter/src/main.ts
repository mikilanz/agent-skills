import { NestFactory, Reflector } from '@nestjs/core';
import { FastifyAdapter, NestFastifyApplication } from '@nestjs/platform-fastify';
import { ClassSerializerInterceptor, ValidationPipe, VersioningType } from '@nestjs/common';
import { DocumentBuilder, SwaggerModule } from '@nestjs/swagger';
import { useContainer } from 'class-validator';
import { AppModule } from './app.module';
import { ConfigService } from './config';
import { ErrorsInterceptor, LoggingInterceptor } from './application/interceptors';
import { GeneralSanitizationPipe } from './application/pipes';

async function bootstrap() {
    const app = await NestFactory.create<NestFastifyApplication>(
        AppModule,
        new FastifyAdapter({ logger: false }),
        {
            logger: ['log', 'error']
        }
    );

    app.useGlobalPipes(
        new GeneralSanitizationPipe({ transform: true }),
        new ValidationPipe({ transform: true })
    );
    app.useGlobalInterceptors(
        new ClassSerializerInterceptor(app.get(Reflector)),
        new LoggingInterceptor(),
        new ErrorsInterceptor()
    );
    app.enableCors({
        origin: true,
        credentials: true
    });
    app.enableVersioning({ type: VersioningType.URI });

    // lets custom class-validator constraints inject providers
    useContainer(app.select(AppModule), { fallbackOnErrors: true });

    const configService = app.get(ConfigService);
    const config = new DocumentBuilder()
        .setTitle('App Name')
        .setDescription('App Name API Description')
        .setVersion(configService.getAppVersion())
        .build();
    const document = SwaggerModule.createDocument(app, config);
    SwaggerModule.setup('api', app, document);

    // 0.0.0.0 so the app is reachable inside containers
    await app.listen(configService.getAppPort(), '0.0.0.0');
}
bootstrap();
