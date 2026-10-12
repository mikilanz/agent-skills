# Templates

A complete example feature, **product**: a tenant-scoped CRUD resource with a dynamic list endpoint and
Redis cache. Every file below was generated with sqlc, compiled, vetted, unit-tested, and exercised
against PostgreSQL 16 and Redis 7 on top of the starter. Copy the shape, rename `product` / `Product`,
and replace the module path `go-api-starter` with the project's.

Files touched:

```
internal/database/migrations/<ts>_create_product_tb.up.sql / .down.sql
internal/adapter/repository/product/sqlc.yaml, query.sql, model/ (generated), repository.go
internal/domain/product/product.go
internal/adapter/http/product/dto/request.go, dto/response.go
internal/usecase/product/service.go
internal/adapter/http/product/handler.go, handler_test.go
cmd/api/main.go                       wiring + route registration
api/docs/                             regenerated with swag
cloudendpoint*.json                   gateway spec entries (see architecture.md)
```

## 1. Migration

`migrate create -ext sql -dir internal/database/migrations -format 20060102150405 create_product_tb`

`internal/database/migrations/20250102000000_create_product_tb.up.sql`
```sql
-- products: one row per product, scoped to a company (tenant).
CREATE TABLE IF NOT EXISTS "product" (
  "id" BIGSERIAL PRIMARY KEY,
  "uuid" UUID NOT NULL DEFAULT gen_random_uuid(),
  "companyCode" VARCHAR(100) NOT NULL,
  "name" VARCHAR(150) NOT NULL,
  "description" TEXT NULL,
  "stock" INT NOT NULL DEFAULT 0,
  "status" VARCHAR(20) NOT NULL DEFAULT 'draft' CHECK ("status" IN ('draft', 'active', 'archived')),
  "metadata" JSONB,                                -- optional structured payload
  "createdBy" VARCHAR(100) NOT NULL,
  "createdAt" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "updatedAt" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT "product_uuid_unique" UNIQUE ("uuid"),
  CONSTRAINT "product_company_name_unique" UNIQUE ("companyCode", "name")
);

CREATE INDEX IF NOT EXISTS "idx_product_company_created_at" ON "product" ("companyCode", "createdAt" DESC);
```

`internal/database/migrations/20250102000000_create_product_tb.down.sql`
```sql
DROP TABLE IF EXISTS "product";
```

Adding a column later is a new migration pair: `ALTER TABLE "product" ADD COLUMN IF NOT EXISTS "sku" VARCHAR(50);`
/ `ALTER TABLE "product" DROP COLUMN IF EXISTS "sku";`, then `sqlc generate` again.

## 2. sqlc

`internal/adapter/repository/product/sqlc.yaml`. `schema` points at the migrations folder (sqlc skips
`.down.sql`); `emit_db_tags` lets the dynamic list scan rows by column name.
```yaml
version: "2"
sql:
  - engine: "postgresql"
    queries: "query.sql"
    schema: "../../../database/migrations"
    gen:
      go:
        package: "model"
        out: "model"
        sql_package: "pgx/v5"
        emit_db_tags: true
```

`internal/adapter/repository/product/query.sql`. Named parameters (`@name`) give readable param
structs; the tenant guard `(@company_code::text = '' OR ...)` lets superadmin pass `""`.
```sql
-- name: GetProductByUuid :one
-- an empty company_code means unscoped (superadmin)
SELECT
  *
FROM "product"
WHERE "uuid" = @uuid AND (@company_code::text = '' OR "companyCode" = @company_code);

-- name: CreateProduct :one
INSERT INTO "product" (
    "companyCode", "name", "description", "stock", "status", "metadata", "createdBy"
) VALUES (
    @company_code, @name, @description, @stock, @status, @metadata::jsonb, @created_by
)
RETURNING *;

-- name: UpdateProduct :one
UPDATE "product"
  SET "name" = @name,
  "description" = @description,
  "stock" = @stock,
  "status" = @status,
  "metadata" = @metadata::jsonb,
  "updatedAt" = NOW()
WHERE "uuid" = @uuid AND (@company_code::text = '' OR "companyCode" = @company_code)
RETURNING *;

-- name: DeleteProduct :execrows
DELETE FROM "product"
WHERE "uuid" = @uuid AND (@company_code::text = '' OR "companyCode" = @company_code);
```

Run `sqlc generate` inside `internal/adapter/repository/product/`. It writes `model/db.go`,
`model/models.go` (`type Product struct` with `pgtype` fields) and `model/query.sql.go`
(`GetProductByUuidParams`, `CreateProductParams`, …). Never edit them by hand.

## 3. Domain

`internal/domain/product/product.go`
```go
package product

import (
	"context"
	"time"

	"go-api-starter/internal/utils"
)

type Product struct {
	ID          int64                  `json:"id"`
	Uuid        string                 `json:"uuid"`
	CompanyCode string                 `json:"companyCode"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Stock       int32                  `json:"stock"`
	Status      string                 `json:"status"`
	Metadata    map[string]interface{} `json:"metadata"`
	CreatedBy   string                 `json:"createdBy"`
	CreatedAt   time.Time              `json:"createdAt"`
	UpdatedAt   time.Time              `json:"updatedAt"`
}

type ProductPagination struct {
	Data  []*Product `json:"data"`
	Total int64      `json:"total"`
	Page  int        `json:"page"`
	Size  int        `json:"size"`
}

// ProductRepository is implemented by adapter/repository/product. An empty companyCode
// means unscoped (superadmin); otherwise rows of other companies are invisible.
type ProductRepository interface {
	GetProductQuery(ctx context.Context, query utils.QueryRequest) (*ProductPagination, error)
	GetProductByUuid(ctx context.Context, uuid string, companyCode string) (*Product, error)
	CreateProduct(ctx context.Context, payload *Product) (*Product, error)
	UpdateProduct(ctx context.Context, payload *Product) (*Product, error)
	DeleteProduct(ctx context.Context, uuid string, companyCode string) error
}
```

## 4. Repository

`internal/adapter/repository/product/repository.go`
```go
package product

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"

	queries "go-api-starter/internal/adapter/repository/product/model"
	"go-api-starter/internal/domain/general"
	domain "go-api-starter/internal/domain/product"
	"go-api-starter/internal/utils"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgtype"
	"github.com/jackc/pgx/v5/pgxpool"
)

// productColumns whitelists the query-string keys the list endpoint may filter and sort on.
var productColumns = utils.Columns("p", "uuid", "companyCode", "name", "stock", "status", "createdBy", "createdAt", "updatedAt")

type ProductRepository struct {
	queries queries.Queries
	db      *pgxpool.Pool
	logger  *slog.Logger
}

func NewRepository(
	dbpool *pgxpool.Pool,
	logger *slog.Logger,
) *ProductRepository {
	queries := queries.New(dbpool)

	return &ProductRepository{
		queries: *queries,
		db:      dbpool,
		logger:  logger,
	}
}

// Helper to map SQLC models to domain models
func (r *ProductRepository) mapProduct(row queries.Product) *domain.Product {
	product := &domain.Product{
		ID:          row.ID,
		Uuid:        row.Uuid.String(),
		CompanyCode: row.CompanyCode,
		Name:        row.Name,
		Description: row.Description.String,
		Stock:       row.Stock,
		Status:      row.Status,
		CreatedBy:   row.CreatedBy,
		CreatedAt:   row.CreatedAt.Time,
		UpdatedAt:   row.UpdatedAt.Time,
	}

	if row.Metadata != nil {
		if err := json.Unmarshal(row.Metadata, &product.Metadata); err != nil {
			r.logger.Error("failed to unmarshal product metadata", slog.String("error", err.Error()), slog.String("uuid", product.Uuid))
		}
	}

	return product
}

func (r *ProductRepository) GetProductQuery(ctx context.Context, query utils.QueryRequest) (*domain.ProductPagination, error) {
	whereClause, filterArgs := utils.FilterData(query.Filters, productColumns)
	sortClause := utils.BuildSortClause(query.Sort, productColumns)
	if sortClause == "" {
		sortClause = ` ORDER BY p."createdAt" DESC`
	}
	// Append pagination args
	args := append(append([]interface{}{}, filterArgs...), query.Size, query.Page*query.Size)
	sql := `SELECT p.* FROM "product" p` + whereClause + sortClause + ` LIMIT ? OFFSET ?`

	rows, err := r.db.Query(ctx, utils.ReplacePlaceholders(sql), args...)
	if err != nil {
		return nil, utils.TranslateDBError(err)
	}
	// emit_db_tags lets pgx map columns to the sqlc model by name
	items, err := pgx.CollectRows(rows, pgx.RowToStructByName[queries.Product])
	if err != nil {
		return nil, utils.TranslateDBError(err)
	}

	// Get total count (run a separate COUNT query with the same WHERE)
	countSQL := `SELECT COUNT(*) FROM "product" p` + whereClause
	var total int64
	if err := r.db.QueryRow(ctx, utils.ReplacePlaceholders(countSQL), filterArgs...).Scan(&total); err != nil {
		return nil, utils.TranslateDBError(err)
	}

	return &domain.ProductPagination{
		Data:  utils.Map(items, r.mapProduct),
		Total: total,
		Page:  query.Page,
		Size:  query.Size,
	}, nil
}

func (r *ProductRepository) GetProductByUuid(ctx context.Context, uuid string, companyCode string) (*domain.Product, error) {
	pgUUID, err := parseUUID(uuid)
	if err != nil {
		return nil, err
	}

	dbResult, err := r.queries.GetProductByUuid(ctx, queries.GetProductByUuidParams{
		Uuid:        pgUUID,
		CompanyCode: companyCode,
	})
	if err != nil {
		return nil, utils.TranslateDBError(err)
	}

	return r.mapProduct(dbResult), nil
}

func (r *ProductRepository) CreateProduct(ctx context.Context, payload *domain.Product) (*domain.Product, error) {
	metadata, err := marshalMetadata(payload.Metadata)
	if err != nil {
		return nil, err
	}

	insertedData, err := r.queries.CreateProduct(ctx, queries.CreateProductParams{
		CompanyCode: payload.CompanyCode,
		Name:        payload.Name,
		Description: pgtype.Text{String: payload.Description, Valid: payload.Description != ""},
		Stock:       payload.Stock,
		Status:      payload.Status,
		Metadata:    metadata,
		CreatedBy:   payload.CreatedBy,
	})
	if err != nil {
		return nil, utils.TranslateDBError(err)
	}

	return r.mapProduct(insertedData), nil
}

func (r *ProductRepository) UpdateProduct(ctx context.Context, payload *domain.Product) (*domain.Product, error) {
	pgUUID, err := parseUUID(payload.Uuid)
	if err != nil {
		return nil, err
	}
	metadata, err := marshalMetadata(payload.Metadata)
	if err != nil {
		return nil, err
	}

	updatedData, err := r.queries.UpdateProduct(ctx, queries.UpdateProductParams{
		Uuid:        pgUUID,
		CompanyCode: payload.CompanyCode,
		Name:        payload.Name,
		Description: pgtype.Text{String: payload.Description, Valid: payload.Description != ""},
		Stock:       payload.Stock,
		Status:      payload.Status,
		Metadata:    metadata,
	})
	if err != nil {
		// pgx.ErrNoRows: no product with this uuid in this company
		return nil, utils.TranslateDBError(err)
	}

	return r.mapProduct(updatedData), nil
}

func (r *ProductRepository) DeleteProduct(ctx context.Context, uuid string, companyCode string) error {
	pgUUID, err := parseUUID(uuid)
	if err != nil {
		return err
	}

	affected, err := r.queries.DeleteProduct(ctx, queries.DeleteProductParams{
		Uuid:        pgUUID,
		CompanyCode: companyCode,
	})
	if err != nil {
		return utils.TranslateDBError(err)
	}
	if affected == 0 {
		return general.ErrNotFound
	}

	return nil
}

func parseUUID(value string) (pgtype.UUID, error) {
	u, err := uuid.Parse(value)
	if err != nil {
		return pgtype.UUID{}, fmt.Errorf("%w: invalid uuid %q", general.ErrBadRequest, value)
	}
	return pgtype.UUID{Bytes: u, Valid: true}, nil
}

// nil metadata stays SQL NULL instead of the JSON literal null
func marshalMetadata(metadata map[string]interface{}) ([]byte, error) {
	if metadata == nil {
		return nil, nil
	}
	return json.Marshal(metadata)
}
```

## 5. DTOs

`internal/adapter/http/product/dto/request.go`
```go
package dto

type ProductRequest struct {
	CompanyCode string                 `json:"companyCode"` // honoured for superadmin only
	Name        string                 `json:"name" binding:"required,max=150"`
	Description string                 `json:"description"`
	Stock       int32                  `json:"stock" binding:"gte=0"`
	Status      string                 `json:"status" binding:"required,oneof=draft active archived"`
	Metadata    map[string]interface{} `json:"metadata"`
}

type UpdateProductRequest struct {
	Uuid string `json:"uuid" binding:"required,uuid"`
	ProductRequest
}

type DeleteProductRequest struct {
	Uuid string `json:"uuid" binding:"required,uuid"`
}
```

`internal/adapter/http/product/dto/response.go`
```go
package dto

import "time"

type ProductResponse struct {
	Uuid        string                 `json:"uuid"`
	CompanyCode string                 `json:"companyCode"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Stock       int32                  `json:"stock"`
	Status      string                 `json:"status"`
	Metadata    map[string]interface{} `json:"metadata"`
	CreatedBy   string                 `json:"createdBy"`
	CreatedAt   time.Time              `json:"createdAt"`
	UpdatedAt   time.Time              `json:"updatedAt"`
}
```

## 6. Service

`internal/usecase/product/service.go`
```go
package product

import (
	"context"
	"crypto/sha1"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	"go-api-starter/internal/adapter/http/product/dto"
	authDomain "go-api-starter/internal/domain/auth"
	"go-api-starter/internal/domain/general"
	domain "go-api-starter/internal/domain/product"
	"go-api-starter/internal/utils"

	"github.com/redis/go-redis/v9"
)

const productListCacheTTL = 10 * time.Minute

type ServiceInterface interface {
	GetProducts(ctx context.Context, claim authDomain.Claims, query utils.QueryRequest) (*domain.ProductPagination, error)
	GetProductByUuid(ctx context.Context, claim authDomain.Claims, uuid string) (*domain.Product, error)
	CreateProduct(ctx context.Context, claim authDomain.Claims, payload *dto.ProductRequest) (*domain.Product, error)
	UpdateProduct(ctx context.Context, claim authDomain.Claims, payload *dto.UpdateProductRequest) (*domain.Product, error)
	DeleteProduct(ctx context.Context, claim authDomain.Claims, payload *dto.DeleteProductRequest) error
}

type Service struct {
	rdb         *redis.Client
	productRepo domain.ProductRepository
	logger      *slog.Logger
}

func NewService(
	rdb *redis.Client,
	productRepo domain.ProductRepository,
	logger *slog.Logger,
) *Service {
	return &Service{
		rdb:         rdb,
		productRepo: productRepo,
		logger:      logger,
	}
}

func (s *Service) GetProducts(ctx context.Context, claim authDomain.Claims, query utils.QueryRequest) (*domain.ProductPagination, error) {
	// superadmin sees every company; everyone else is pinned to their own
	companyCode := claim.ScopedCompanyCode("")
	if companyCode != "" {
		query.Filters = append(query.Filters, utils.QueryFilter{
			Key:    "companyCode",
			Value:  companyCode,
			Filter: utils.FilterCriteriaEqual,
		})
	}

	cacheKey := productListCacheKey(companyCode, query)
	cache, errCache := s.rdb.Get(ctx, cacheKey).Result()
	if errCache == nil && cache != "" {
		var pagination domain.ProductPagination
		if errDecode := json.Unmarshal([]byte(cache), &pagination); errDecode != nil {
			s.logger.Error("failed to decode product list from cache", slog.String("error", errDecode.Error()))
		} else {
			return &pagination, nil
		}
	} else if errCache != nil && errCache != redis.Nil {
		s.logger.Error("failed to get product list from redis", slog.String("error", errCache.Error()))
	}

	result, err := s.productRepo.GetProductQuery(ctx, query)
	if err != nil {
		s.logger.Error("failed to get products", slog.String("error", err.Error()))
		return nil, err
	}

	marshaled, errMarshal := json.Marshal(result)
	if errMarshal != nil {
		s.logger.Error("failed to marshal product list for cache", slog.String("error", errMarshal.Error()))
	} else if errSet := s.rdb.Set(ctx, cacheKey, marshaled, productListCacheTTL).Err(); errSet != nil {
		s.logger.Error("failed to set product list to redis", slog.String("error", errSet.Error()))
	}

	return result, nil
}

func (s *Service) GetProductByUuid(ctx context.Context, claim authDomain.Claims, uuid string) (*domain.Product, error) {
	product, err := s.productRepo.GetProductByUuid(ctx, uuid, claim.ScopedCompanyCode(""))
	if err != nil {
		s.logger.Error("failed to get product", slog.String("error", err.Error()), slog.String("uuid", uuid))
		return nil, err
	}

	return product, nil
}

func (s *Service) CreateProduct(ctx context.Context, claim authDomain.Claims, payload *dto.ProductRequest) (*domain.Product, error) {
	companyCode := claim.ScopedCompanyCode(payload.CompanyCode)
	if companyCode == "" {
		return nil, fmt.Errorf("%w: companyCode is required", general.ErrBadRequest)
	}

	product, err := s.productRepo.CreateProduct(ctx, &domain.Product{
		CompanyCode: companyCode,
		Name:        payload.Name,
		Description: payload.Description,
		Stock:       payload.Stock,
		Status:      payload.Status,
		Metadata:    payload.Metadata,
		CreatedBy:   claim.Username,
	})
	if err != nil {
		s.logger.Error("failed to create product", slog.String("error", err.Error()))
		return nil, err
	}

	s.invalidateProductListCache(ctx, product.CompanyCode)

	return product, nil
}

func (s *Service) UpdateProduct(ctx context.Context, claim authDomain.Claims, payload *dto.UpdateProductRequest) (*domain.Product, error) {
	product, err := s.productRepo.UpdateProduct(ctx, &domain.Product{
		Uuid:        payload.Uuid,
		CompanyCode: claim.ScopedCompanyCode(""),
		Name:        payload.Name,
		Description: payload.Description,
		Stock:       payload.Stock,
		Status:      payload.Status,
		Metadata:    payload.Metadata,
	})
	if err != nil {
		s.logger.Error("failed to update product", slog.String("error", err.Error()), slog.String("uuid", payload.Uuid))
		return nil, err
	}

	s.invalidateProductListCache(ctx, product.CompanyCode)

	return product, nil
}

func (s *Service) DeleteProduct(ctx context.Context, claim authDomain.Claims, payload *dto.DeleteProductRequest) error {
	companyCode := claim.ScopedCompanyCode("")

	product, err := s.productRepo.GetProductByUuid(ctx, payload.Uuid, companyCode)
	if err != nil {
		s.logger.Error("failed to get product to delete", slog.String("error", err.Error()), slog.String("uuid", payload.Uuid))
		return err
	}

	if err := s.productRepo.DeleteProduct(ctx, payload.Uuid, companyCode); err != nil {
		s.logger.Error("failed to delete product", slog.String("error", err.Error()), slog.String("uuid", payload.Uuid))
		return err
	}

	s.invalidateProductListCache(ctx, product.CompanyCode)

	return nil
}

// productListCacheKey is shared by Get and Set so both always use the same key; the
// hash covers filters, sort and paging so different queries never share a cache entry.
func productListCacheKey(companyCode string, query utils.QueryRequest) string {
	if companyCode == "" {
		companyCode = "all"
	}
	raw, _ := json.Marshal(query)
	sum := sha1.Sum(raw)
	return fmt.Sprintf("product:list:%s:%s", companyCode, hex.EncodeToString(sum[:]))
}

// invalidateProductListCache drops the company's cached lists and the superadmin "all" lists.
func (s *Service) invalidateProductListCache(ctx context.Context, companyCode string) {
	for _, pattern := range []string{
		fmt.Sprintf("product:list:%s:*", companyCode),
		"product:list:all:*",
	} {
		iter := s.rdb.Scan(ctx, 0, pattern, 100).Iterator()
		for iter.Next(ctx) {
			if errDel := s.rdb.Del(ctx, iter.Val()).Err(); errDel != nil {
				s.logger.Error("failed to delete product list cache", slog.String("error", errDel.Error()))
			}
		}
		if err := iter.Err(); err != nil {
			s.logger.Error("failed to iterate redis keys for product list cache invalidation", slog.String("error", err.Error()))
		}
	}
}
```

## 7. Handler

`internal/adapter/http/product/handler.go`
```go
package product

import (
	"log/slog"
	"net/http"
	"strconv"

	"go-api-starter/internal/adapter/http/product/dto"
	"go-api-starter/internal/application/middleware"
	authDomain "go-api-starter/internal/domain/auth"
	"go-api-starter/internal/domain/general"
	domain "go-api-starter/internal/domain/product"
	usecase "go-api-starter/internal/usecase/product"
	"go-api-starter/internal/utils"

	"github.com/gin-gonic/gin"
)

type Handler struct {
	svc    usecase.ServiceInterface
	logger *slog.Logger
}

func NewHandler(
	svc usecase.ServiceInterface,
	logger *slog.Logger,
) *Handler {
	return &Handler{
		svc:    svc,
		logger: logger,
	}
}

func (h *Handler) RegisterRoutes(rg *gin.RouterGroup) {
	group := rg.Group("/product")

	group.GET("",
		middleware.RoleGuard(authDomain.RoleSuperadmin, authDomain.RoleAdmin, authDomain.RoleEmployee, authDomain.RoleUser),
		h.getProducts,
	)

	group.GET("/:uuid",
		middleware.RoleGuard(authDomain.RoleSuperadmin, authDomain.RoleAdmin, authDomain.RoleEmployee, authDomain.RoleUser),
		h.getProductByUuid,
	)

	group.POST("",
		middleware.RoleGuard(authDomain.RoleSuperadmin, authDomain.RoleAdmin, authDomain.RoleEmployee),
		h.createProduct,
	)

	group.PUT("",
		middleware.RoleGuard(authDomain.RoleSuperadmin, authDomain.RoleAdmin, authDomain.RoleEmployee),
		h.updateProduct,
	)

	group.DELETE("",
		middleware.RoleGuard(authDomain.RoleSuperadmin, authDomain.RoleAdmin),
		h.deleteProduct,
	)
}

func toProductResponse(p *domain.Product) dto.ProductResponse {
	return dto.ProductResponse{
		Uuid:        p.Uuid,
		CompanyCode: p.CompanyCode,
		Name:        p.Name,
		Description: p.Description,
		Stock:       p.Stock,
		Status:      p.Status,
		Metadata:    p.Metadata,
		CreatedBy:   p.CreatedBy,
		CreatedAt:   p.CreatedAt,
		UpdatedAt:   p.UpdatedAt,
	}
}

// Get products
//
//	@Summary	Retrieve products
//	@Schemes
//	@Description	Retrieve products of the caller's company. Paginate with page (zero-based) and size, sort with sort=field,asc|desc, filter with field.criteria=value (equal, notEqual, greaterThan, greaterThanOrEqual, lessThan, lessThanOrEqual, includes, contains, in, notIn, between). The total count is returned in the x-total-count header.
//	@Tags			Product
//	@Accept			json
//	@Produce		json
//	@Param			x-endpoint-api-userinfo	header		string	true	"x-endpoint-api-userinfo"
//	@Param			page						query		int		false	"page number, zero-based"
//	@Param			size						query		int		false	"page size"
//	@Param			sort						query		string	false	"e.g. createdAt,desc"
//	@Success		200							{array}		dto.ProductResponse
//	@Header			200							{string}	x-total-count	"total rows matching the filters"
//	@Failure		401							{object}	general.DefaultHttpResponse
//	@Failure		403							{object}	general.DefaultHttpResponse
//	@Security		BearerAuth
//	@Router			/api/v1/product [get]
func (h *Handler) getProducts(c *gin.Context) {
	claim, ok := middleware.GetClaims(c)
	if !ok {
		c.JSON(http.StatusUnauthorized, general.DefaultHttpResponse{
			StatusCode: http.StatusUnauthorized,
			Message:    "Unauthorized",
			Ok:         false,
		})
		return
	}

	resultData, err := h.svc.GetProducts(c, claim, middleware.GetQueryRequest(c))
	if err != nil {
		status := utils.HttpStatusFromError(err)
		c.JSON(status, general.DefaultHttpResponse{
			StatusCode: status,
			Message:    "Failed to get products",
			Ok:         false,
		})
		return
	}

	c.Header("x-total-count", strconv.FormatInt(resultData.Total, 10))
	c.JSON(http.StatusOK, utils.Map(resultData.Data, toProductResponse))
}

// Get product by uuid
//
//	@Summary	Retrieve a product
//	@Schemes
//	@Description	Retrieve a product by uuid
//	@Tags			Product
//	@Accept			json
//	@Produce		json
//	@Param			x-endpoint-api-userinfo	header		string	true	"x-endpoint-api-userinfo"
//	@Param			uuid						path		string	true	"product uuid"
//	@Success		200							{object}	dto.ProductResponse
//	@Failure		401							{object}	general.DefaultHttpResponse
//	@Failure		403							{object}	general.DefaultHttpResponse
//	@Failure		404							{object}	general.DefaultHttpResponse
//	@Security		BearerAuth
//	@Router			/api/v1/product/{uuid} [get]
func (h *Handler) getProductByUuid(c *gin.Context) {
	claim, ok := middleware.GetClaims(c)
	if !ok {
		c.JSON(http.StatusUnauthorized, general.DefaultHttpResponse{
			StatusCode: http.StatusUnauthorized,
			Message:    "Unauthorized",
			Ok:         false,
		})
		return
	}

	resultData, err := h.svc.GetProductByUuid(c, claim, c.Param("uuid"))
	if err != nil {
		status := utils.HttpStatusFromError(err)
		c.JSON(status, general.DefaultHttpResponse{
			StatusCode: status,
			Message:    "Failed to get product",
			Ok:         false,
		})
		return
	}

	c.JSON(http.StatusOK, toProductResponse(resultData))
}

// Create product
//
//	@Summary	Create product
//	@Schemes
//	@Description	Create product. companyCode in the body is honoured for superadmin only.
//	@Tags			Product
//	@Accept			json
//	@Produce		json
//	@Param			x-endpoint-api-userinfo	header		string				true	"x-endpoint-api-userinfo"
//	@Param			request						body		dto.ProductRequest	true	"required params"
//	@Success		200							{object}	dto.ProductResponse
//	@Failure		400							{object}	general.DefaultHttpResponse
//	@Failure		401							{object}	general.DefaultHttpResponse
//	@Failure		403							{object}	general.DefaultHttpResponse
//	@Failure		409							{object}	general.DefaultHttpResponse
//	@Security		BearerAuth
//	@Router			/api/v1/product [post]
func (h *Handler) createProduct(c *gin.Context) {
	claim, ok := middleware.GetClaims(c)
	if !ok {
		c.JSON(http.StatusUnauthorized, general.DefaultHttpResponse{
			StatusCode: http.StatusUnauthorized,
			Message:    "Unauthorized",
			Ok:         false,
		})
		return
	}

	var req dto.ProductRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, general.DefaultHttpResponse{
			StatusCode: http.StatusBadRequest,
			Message:    err.Error(),
			Ok:         false,
		})
		return
	}

	resultData, err := h.svc.CreateProduct(c, claim, &req)
	if err != nil {
		status := utils.HttpStatusFromError(err)
		c.JSON(status, general.DefaultHttpResponse{
			StatusCode: status,
			Message:    "Failed to create product",
			Ok:         false,
		})
		return
	}

	c.JSON(http.StatusOK, toProductResponse(resultData))
}

// Update product
//
//	@Summary	Update product
//	@Schemes
//	@Description	Update product
//	@Tags			Product
//	@Accept			json
//	@Produce		json
//	@Param			x-endpoint-api-userinfo	header		string						true	"x-endpoint-api-userinfo"
//	@Param			request						body		dto.UpdateProductRequest	true	"required params"
//	@Success		200							{object}	dto.ProductResponse
//	@Failure		400							{object}	general.DefaultHttpResponse
//	@Failure		401							{object}	general.DefaultHttpResponse
//	@Failure		403							{object}	general.DefaultHttpResponse
//	@Failure		404							{object}	general.DefaultHttpResponse
//	@Security		BearerAuth
//	@Router			/api/v1/product [put]
func (h *Handler) updateProduct(c *gin.Context) {
	claim, ok := middleware.GetClaims(c)
	if !ok {
		c.JSON(http.StatusUnauthorized, general.DefaultHttpResponse{
			StatusCode: http.StatusUnauthorized,
			Message:    "Unauthorized",
			Ok:         false,
		})
		return
	}

	var req dto.UpdateProductRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, general.DefaultHttpResponse{
			StatusCode: http.StatusBadRequest,
			Message:    err.Error(),
			Ok:         false,
		})
		return
	}

	resultData, err := h.svc.UpdateProduct(c, claim, &req)
	if err != nil {
		status := utils.HttpStatusFromError(err)
		c.JSON(status, general.DefaultHttpResponse{
			StatusCode: status,
			Message:    "Failed to update product",
			Ok:         false,
		})
		return
	}

	c.JSON(http.StatusOK, toProductResponse(resultData))
}

// Delete product
//
//	@Summary	Delete product
//	@Schemes
//	@Description	Delete product
//	@Tags			Product
//	@Accept			json
//	@Produce		json
//	@Param			x-endpoint-api-userinfo	header		string						true	"x-endpoint-api-userinfo"
//	@Param			request						body		dto.DeleteProductRequest	true	"required params"
//	@Success		200							{object}	general.DefaultHttpResponse
//	@Failure		400							{object}	general.DefaultHttpResponse
//	@Failure		401							{object}	general.DefaultHttpResponse
//	@Failure		403							{object}	general.DefaultHttpResponse
//	@Failure		404							{object}	general.DefaultHttpResponse
//	@Security		BearerAuth
//	@Router			/api/v1/product [delete]
func (h *Handler) deleteProduct(c *gin.Context) {
	claim, ok := middleware.GetClaims(c)
	if !ok {
		c.JSON(http.StatusUnauthorized, general.DefaultHttpResponse{
			StatusCode: http.StatusUnauthorized,
			Message:    "Unauthorized",
			Ok:         false,
		})
		return
	}

	var req dto.DeleteProductRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, general.DefaultHttpResponse{
			StatusCode: http.StatusBadRequest,
			Message:    err.Error(),
			Ok:         false,
		})
		return
	}

	if err := h.svc.DeleteProduct(c, claim, &req); err != nil {
		status := utils.HttpStatusFromError(err)
		c.JSON(status, general.DefaultHttpResponse{
			StatusCode: status,
			Message:    "Failed to delete product",
			Ok:         false,
		})
		return
	}

	c.JSON(http.StatusOK, general.DefaultHttpResponse{
		StatusCode: http.StatusOK,
		Message:    "success delete product",
		Ok:         true,
	})
}
```

## 8. Handler tests

`internal/adapter/http/product/handler_test.go`. Tests live in the handler's package so they can call
the unexported methods directly; the mock implements every `ServiceInterface` method.
```go
package product

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"go-api-starter/internal/adapter/http/product/dto"
	authDomain "go-api-starter/internal/domain/auth"
	"go-api-starter/internal/domain/general"
	domain "go-api-starter/internal/domain/product"
	"go-api-starter/internal/utils"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
)

// MockService is a mock implementation of usecase.ServiceInterface
type MockService struct {
	mock.Mock
}

func (m *MockService) GetProducts(ctx context.Context, claim authDomain.Claims, query utils.QueryRequest) (*domain.ProductPagination, error) {
	args := m.Called(ctx, claim, query)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*domain.ProductPagination), args.Error(1)
}

func (m *MockService) GetProductByUuid(ctx context.Context, claim authDomain.Claims, uuid string) (*domain.Product, error) {
	args := m.Called(ctx, claim, uuid)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*domain.Product), args.Error(1)
}

func (m *MockService) CreateProduct(ctx context.Context, claim authDomain.Claims, payload *dto.ProductRequest) (*domain.Product, error) {
	args := m.Called(ctx, claim, payload)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*domain.Product), args.Error(1)
}

func (m *MockService) UpdateProduct(ctx context.Context, claim authDomain.Claims, payload *dto.UpdateProductRequest) (*domain.Product, error) {
	args := m.Called(ctx, claim, payload)
	if args.Get(0) == nil {
		return nil, args.Error(1)
	}
	return args.Get(0).(*domain.Product), args.Error(1)
}

func (m *MockService) DeleteProduct(ctx context.Context, claim authDomain.Claims, payload *dto.DeleteProductRequest) error {
	args := m.Called(ctx, claim, payload)
	return args.Error(0)
}

func TestHandler_getProducts(t *testing.T) {
	gin.SetMode(gin.TestMode)

	mockSvc := new(MockService)
	handler := NewHandler(mockSvc, nil)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	claims := authDomain.Claims{CompanyCode: "test-company", Role: authDomain.RoleAdmin}
	c.Set("userInfo", claims)

	expected := &domain.ProductPagination{
		Data:  []*domain.Product{{Uuid: "1", Name: "Widget", CompanyCode: "test-company"}},
		Total: 1,
	}
	mockSvc.On("GetProducts", mock.Anything, claims, mock.Anything).Return(expected, nil)

	handler.getProducts(c)

	assert.Equal(t, http.StatusOK, w.Code)
	assert.Equal(t, "1", w.Header().Get("x-total-count"))

	var response []dto.ProductResponse
	err := json.Unmarshal(w.Body.Bytes(), &response)
	assert.NoError(t, err)
	assert.Len(t, response, 1)
	assert.Equal(t, "Widget", response[0].Name)

	mockSvc.AssertExpectations(t)
}

func TestHandler_getProducts_Unauthorized(t *testing.T) {
	gin.SetMode(gin.TestMode)

	mockSvc := new(MockService)
	handler := NewHandler(mockSvc, nil)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	handler.getProducts(c)

	assert.Equal(t, http.StatusUnauthorized, w.Code)
	mockSvc.AssertNotCalled(t, "GetProducts")
}

func TestHandler_getProductByUuid_NotFound(t *testing.T) {
	gin.SetMode(gin.TestMode)

	mockSvc := new(MockService)
	handler := NewHandler(mockSvc, nil)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	claims := authDomain.Claims{CompanyCode: "test-company", Role: authDomain.RoleAdmin}
	c.Set("userInfo", claims)
	c.Params = gin.Params{{Key: "uuid", Value: "missing"}}

	mockSvc.On("GetProductByUuid", mock.Anything, claims, "missing").Return(nil, general.ErrNotFound)

	handler.getProductByUuid(c)

	assert.Equal(t, http.StatusNotFound, w.Code)
	mockSvc.AssertExpectations(t)
}

func TestHandler_createProduct(t *testing.T) {
	gin.SetMode(gin.TestMode)

	mockSvc := new(MockService)
	handler := NewHandler(mockSvc, nil)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	claims := authDomain.Claims{Username: "testuser", CompanyCode: "test-company", Role: authDomain.RoleAdmin}
	c.Set("userInfo", claims)

	req := dto.ProductRequest{Name: "Widget", Stock: 5, Status: "active"}
	reqBody, _ := json.Marshal(req)
	c.Request = httptest.NewRequest(http.MethodPost, "/product", bytes.NewBuffer(reqBody))
	c.Request.Header.Set("Content-Type", "application/json")

	expected := &domain.Product{Uuid: "1", Name: "Widget", Stock: 5, Status: "active", CompanyCode: "test-company", CreatedBy: "testuser"}
	mockSvc.On("CreateProduct", mock.Anything, claims, &req).Return(expected, nil)

	handler.createProduct(c)

	assert.Equal(t, http.StatusOK, w.Code)

	var response dto.ProductResponse
	err := json.Unmarshal(w.Body.Bytes(), &response)
	assert.NoError(t, err)
	assert.Equal(t, "1", response.Uuid)

	mockSvc.AssertExpectations(t)
}

func TestHandler_createProduct_BadRequest(t *testing.T) {
	gin.SetMode(gin.TestMode)

	mockSvc := new(MockService)
	handler := NewHandler(mockSvc, nil)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Set("userInfo", authDomain.Claims{Role: authDomain.RoleAdmin})

	c.Request = httptest.NewRequest(http.MethodPost, "/product", bytes.NewBufferString(`{"status":"unknown"}`))
	c.Request.Header.Set("Content-Type", "application/json")

	handler.createProduct(c)

	assert.Equal(t, http.StatusBadRequest, w.Code)
	mockSvc.AssertNotCalled(t, "CreateProduct")
}

func TestHandler_deleteProduct(t *testing.T) {
	gin.SetMode(gin.TestMode)

	mockSvc := new(MockService)
	handler := NewHandler(mockSvc, nil)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	claims := authDomain.Claims{CompanyCode: "test-company", Role: authDomain.RoleAdmin}
	c.Set("userInfo", claims)

	req := dto.DeleteProductRequest{Uuid: "7b0e7c4e-3f7c-4d43-9a43-2f0a1c9b6d11"}
	reqBody, _ := json.Marshal(req)
	c.Request = httptest.NewRequest(http.MethodDelete, "/product", bytes.NewBuffer(reqBody))
	c.Request.Header.Set("Content-Type", "application/json")

	mockSvc.On("DeleteProduct", mock.Anything, claims, &req).Return(nil)

	handler.deleteProduct(c)

	assert.Equal(t, http.StatusOK, w.Code)

	var response general.DefaultHttpResponse
	err := json.Unmarshal(w.Body.Bytes(), &response)
	assert.NoError(t, err)
	assert.Equal(t, "success delete product", response.Message)

	mockSvc.AssertExpectations(t)
}
```

## 9. Wiring in `cmd/api/main.go`

Imports (kept with the other handler imports, gofmt-sorted):
```go
	healthHandler "go-api-starter/internal/adapter/http/health"
	productHandler "go-api-starter/internal/adapter/http/product"
	swaggerHandler "go-api-starter/internal/adapter/http/swagger"
	productRepo "go-api-starter/internal/adapter/repository/product"
	productUsecase "go-api-starter/internal/usecase/product"
```

Dependency injection block, repositories first:
```go
	// Dependency Injection: repositories -> services -> handlers
	var (
		productRepository = productRepo.NewRepository(dbpool, log)

		productService = productUsecase.NewService(rdb, productRepository, log)

		productHandler = productHandler.NewHandler(productService, log)
		healthHandler  = healthHandler.NewHandler(dbpool, rdb, log)
		swaggerHandler = swaggerHandler.NewHandler(log)
	)
```

Routes:
```go
	apiV1.Use(middleware.ClaimsMiddleware(), middleware.QueryPaginationMiddleware(log))
	{
		productHandler.RegisterRoutes(apiV1)
	}
```

Then regenerate docs (`swag init -g cmd/api/main.go -o api/docs`), add the paths to the gateway spec,
and run `go mod tidy && go vet ./... && go test ./...`.

Exercising it (claims header built by hand, as the gateway would):

```bash
H=$(printf '%s' '{"companyCode":"ACME","role":"admin","username":"jane"}' | base64 -w0)
curl -XPOST localhost:8080/api/v1/product -H "x-endpoint-api-userinfo: $H" -H 'content-type: application/json' \
  -d '{"name":"Widget","stock":5,"status":"active","metadata":{"color":"red"}}'
curl -i "localhost:8080/api/v1/product?sort=stock,desc&name.includes=wid&status.in=draft,active&size=20" -H "x-endpoint-api-userinfo: $H"
```

---

# Snippets

## Adding an env var

`internal/config/config.go`: a field under its section comment, read in `InitConfig`.
```go
	// Auth (service-to-service)
	AUTH_URL         string
	AUTH_USERNAME    string
	AUTH_PASSWORD    string
	AUTH_BASIC_TOKEN string

	// External Services
	USER_SERVICE_BASE_URL string
```

```go
			AUTH_URL:                getEnvOrFatal("AUTH_URL"),
			AUTH_USERNAME:           getEnvOrFatal("AUTH_USERNAME"),
			AUTH_PASSWORD:           getEnvOrFatal("AUTH_PASSWORD"),
			AUTH_BASIC_TOKEN:        getEnvOrFatal("AUTH_BASIC_TOKEN"),
			USER_SERVICE_BASE_URL:   getEnvOrFatal("USER_SERVICE_BASE_URL"),
```

Add the keys (empty, or with a safe local value) to `.env.example` and to the deployment manifests.

## Outbound integration with a cached machine token

`internal/adapter/http/auth/dto/response.go`
```go
package dto

type JwtAuthResponse struct {
	AccessToken string `json:"access_token"`
	TokenType   string `json:"token_type"`
	ExpiresIn   int    `json:"expires_in"`
	IdToken     string `json:"id_token"`
}
```

`internal/domain/general/external-user.go` (or the upstream entity's own domain package)
```go
package general

type ExternalUser struct {
	Uuid     string `json:"uuid"`
	Username string `json:"username"`
	Email    string `json:"email"`
}
```

`internal/usecase/external-user/service.go`
```go
package externaluser

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"

	"go-api-starter/internal/adapter/http/auth/dto"
	"go-api-starter/internal/config"
	"go-api-starter/internal/domain/general"
	httpUsecase "go-api-starter/internal/usecase/http"

	"github.com/redis/go-redis/v9"
)

const (
	serviceTokenCacheKey = "external_user:service_token"
	userPageSize         = 500
)

type Service struct {
	rdb         *redis.Client
	httpService *httpUsecase.Service
	logger      *slog.Logger
}

func NewService(
	rdb *redis.Client,
	httpService *httpUsecase.Service,
	logger *slog.Logger,
) *Service {
	return &Service{
		rdb:         rdb,
		httpService: httpService,
		logger:      logger,
	}
}

// GetServiceToken returns a machine token for service-to-service calls, cached in Redis
// a little shorter than its lifetime.
func (s *Service) GetServiceToken(ctx context.Context) (*dto.JwtAuthResponse, error) {
	cache, err := s.rdb.Get(ctx, serviceTokenCacheKey).Result()
	if err == nil && cache != "" {
		var authDTO dto.JwtAuthResponse
		if errDecode := json.Unmarshal([]byte(cache), &authDTO); errDecode == nil {
			return &authDTO, nil
		}
		s.logger.Error("failed to decode service token from cache")
	}

	cfg := config.GetConfig()
	form := url.Values{}
	form.Add("username", cfg.AUTH_USERNAME)
	form.Add("password", cfg.AUTH_PASSWORD)
	form.Add("grant_type", "password")

	var authDTO dto.JwtAuthResponse
	_, err = s.httpService.MakeJSONRequest(ctx, &general.HttpOptions{
		Method: http.MethodPost,
		URL:    fmt.Sprintf("%s/v1/oauth/tokens", cfg.AUTH_URL),
		Headers: http.Header{
			"Content-Type":  {"application/x-www-form-urlencoded"},
			"Authorization": {fmt.Sprintf("Basic %s", cfg.AUTH_BASIC_TOKEN)},
		},
		Body: strings.NewReader(form.Encode()),
	}, &authDTO)
	if err != nil {
		s.logger.Error("failed to get service token", slog.String("error", err.Error()))
		return nil, err
	}

	// cache only a successfully decoded token
	authBytes, _ := json.Marshal(authDTO)
	if errSet := s.rdb.Set(ctx, serviceTokenCacheKey, authBytes, 55*time.Minute).Err(); errSet != nil {
		s.logger.Error("failed to set service token to redis", slog.String("error", errSet.Error()))
	}

	return &authDTO, nil
}

// GetAllUsers pages through the upstream list endpoint using its x-total-count header.
func (s *Service) GetAllUsers(ctx context.Context, companyCode string) ([]general.ExternalUser, error) {
	token, err := s.GetServiceToken(ctx)
	if err != nil {
		return nil, err
	}
	cfg := config.GetConfig()

	var allUsers []general.ExternalUser
	for page := 0; ; page++ {
		var pageUsers []general.ExternalUser
		headers, err := s.httpService.MakeJSONRequest(ctx, &general.HttpOptions{
			Method: http.MethodGet,
			URL: fmt.Sprintf("%s/proxy/%s/api/users?size=%d&page=%d",
				cfg.USER_SERVICE_BASE_URL, url.PathEscape(companyCode), userPageSize, page),
			Headers: http.Header{
				"Content-Type":  {"application/json"},
				"Authorization": {"Bearer " + token.IdToken},
			},
		}, &pageUsers)
		if err != nil {
			s.logger.Error("failed to get users page", slog.Int("page", page), slog.String("error", err.Error()))
			return nil, err
		}

		allUsers = append(allUsers, pageUsers...)
		total, _ := strconv.Atoi(headers.Get("x-total-count"))
		if len(pageUsers) < userPageSize || len(allUsers) >= total {
			break
		}
	}

	s.logger.Info("finished fetching users", slog.Int("total", len(allUsers)))
	return allUsers, nil
}
```

Wire it as `externalUserService = externalUserUsecase.NewService(rdb, httpService, log)` with
`httpService = httpUsecase.NewService(log)` first, and inject the concrete `*externalUser.Service` into
the services that need it.

## Bulk insert with COPY inside a transaction

Add to `query.sql` (`:copyfrom` generates `model/copyfrom.go`; it needs plain params, so no `::jsonb` cast):
```sql
-- name: BulkCreateProduct :copyfrom
INSERT INTO "product" (
    "companyCode", "name", "description", "stock", "status", "metadata", "createdBy"
) VALUES (
    @company_code, @name, @description, @stock, @status, @metadata, @created_by
);
```

Add to the repository (and the method to the domain interface):
```go
// BulkCreateProducts inserts all rows with COPY inside one transaction: either every
// row lands or none does.
func (r *ProductRepository) BulkCreateProducts(ctx context.Context, payload []*domain.Product) (int64, error) {
	args := make([]queries.BulkCreateProductParams, len(payload))
	for i, p := range payload {
		metadata, err := marshalMetadata(p.Metadata)
		if err != nil {
			return 0, err
		}
		args[i] = queries.BulkCreateProductParams{
			CompanyCode: p.CompanyCode,
			Name:        p.Name,
			Description: pgtype.Text{String: p.Description, Valid: p.Description != ""},
			Stock:       p.Stock,
			Status:      p.Status,
			Metadata:    metadata,
			CreatedBy:   p.CreatedBy,
		}
	}

	var inserted int64
	err := pgx.BeginFunc(ctx, r.db, func(tx pgx.Tx) error {
		n, err := r.queries.WithTx(tx).BulkCreateProduct(ctx, args)
		inserted = n
		return err
	})
	if err != nil {
		return 0, utils.TranslateDBError(err)
	}

	return inserted, nil
}
```

For a multi-statement transaction across queries, use the same `pgx.BeginFunc` +
`r.queries.WithTx(tx)` and run every call on that `*Queries`. A transaction spanning two
repositories belongs in one repository method that uses both sqlc packages with the same `tx`.

## Background job with a worker pool

`internal/usecase/product/import.go`. The handler gets 200 immediately; the work runs detached from
the request context.
```go
package product

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
	"sync/atomic"

	"go-api-starter/internal/adapter/http/product/dto"
	authDomain "go-api-starter/internal/domain/auth"
	"go-api-starter/internal/domain/general"
	domain "go-api-starter/internal/domain/product"
)

const (
	importBatchSize   = 1000 // rows per COPY
	importWorkerCount = 4    // parallel batches
)

// ImportProducts returns immediately and imports in the background.
func (s *Service) ImportProducts(ctx context.Context, claim authDomain.Claims, payload []dto.ProductRequest) error {
	products := make([]*domain.Product, len(payload))
	for i, p := range payload {
		companyCode := claim.ScopedCompanyCode(p.CompanyCode)
		if companyCode == "" {
			return fmt.Errorf("%w: companyCode is required (row %d)", general.ErrBadRequest, i)
		}
		products[i] = &domain.Product{
			CompanyCode: companyCode,
			Name:        p.Name,
			Description: p.Description,
			Stock:       p.Stock,
			Status:      p.Status,
			Metadata:    p.Metadata,
			CreatedBy:   claim.Username,
		}
	}

	// the request context is cancelled when the handler returns, and *gin.Context is
	// reused for the next request: detach before handing it to a goroutine
	go s.importProducts(context.WithoutCancel(ctx), products)

	return nil
}

func (s *Service) importProducts(ctx context.Context, products []*domain.Product) {
	batches := make(chan []*domain.Product, importWorkerCount)
	var processed, failed atomic.Int64
	var wg sync.WaitGroup

	// Start workers
	for range importWorkerCount {
		wg.Go(func() {
			for batch := range batches {
				inserted, err := s.productRepo.BulkCreateProducts(ctx, batch)
				if err != nil {
					s.logger.Error("failed to import product batch", slog.String("error", err.Error()), slog.Int("batchSize", len(batch)))
					failed.Add(int64(len(batch)))
					continue
				}
				processed.Add(inserted)
			}
		})
	}

	// Send batches to workers
	for i := 0; i < len(products); i += importBatchSize {
		batches <- products[i:min(i+importBatchSize, len(products))]
	}
	close(batches)
	wg.Wait()

	companyCodes := map[string]struct{}{}
	for _, p := range products {
		companyCodes[p.CompanyCode] = struct{}{}
	}
	for companyCode := range companyCodes {
		s.invalidateProductListCache(ctx, companyCode)
	}
	s.logger.Info("finished product import", slog.Int64("processed", processed.Load()), slog.Int64("failed", failed.Load()), slog.Int("total", len(products)))
}
```

For user-visible progress, create a job row before starting (status `pending`), update it to
`processing` with totals, add processed/failed counts per batch, and finish with `completed` or
`failed` plus `completedAt` (see architecture.md, "Background work").

## Joined list queries

Merge column maps so filters and sorts can target both tables, and select the extra columns
explicitly into a row struct (sqlc emits one, `Get<X>Row`, if you also define the join in `query.sql`):

```go
var orderColumns = func() map[string]string {
	columns := utils.Columns("o", "companyCode", "status", "total", "createdAt")
	columns["customerName"] = `c."name"`
	return columns
}()

sql := `SELECT o.*, c."name" FROM "order" o
        LEFT JOIN "customer" c ON o."customerId" = c."id"` + whereClause + sortClause + ` LIMIT ? OFFSET ?`
```

Keep the `COUNT(*)` query's FROM/JOIN identical to the data query so the total matches.
