import { BaseEntity, BeforeUpdate, Column, PrimaryGeneratedColumn } from 'typeorm';

export class DefaultEntity extends BaseEntity {
    @PrimaryGeneratedColumn()
    id: number;

    @Column()
    uuid: string;

    @Column({
        type: 'timestamp with time zone',
        default: () => 'CURRENT_TIMESTAMP'
    })
    createdAt: Date;

    @Column({
        type: 'timestamp with time zone',
        default: () => 'CURRENT_TIMESTAMP'
    })
    updatedAt: Date;

    @BeforeUpdate()
    updateTimestamp() {
        this.updatedAt = new Date();
    }
}
