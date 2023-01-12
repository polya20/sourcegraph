package graphqlbackend

import (
	"context"

	"github.com/google/go-cmp/cmp"
	"github.com/graph-gophers/graphql-go"
	"github.com/graph-gophers/graphql-go/relay"
	"github.com/sourcegraph/sourcegraph/internal/database"
	"github.com/sourcegraph/sourcegraph/internal/gqlutil"
)

const siteConfigurationKind = "SiteConfiguration"

type SiteConfigurationChangeResolver struct {
	db                 database.DB
	siteConfig         *database.SiteConfig
	previousSiteConfig *database.SiteConfig
}

func (r SiteConfigurationChangeResolver) ID() graphql.ID {
	return relay.MarshalID(siteConfigurationKind, r.siteConfig.ID)
}

func (r SiteConfigurationChangeResolver) PreviousID() *graphql.ID {
	if r.previousSiteConfig != nil {
		id := relay.MarshalID(siteConfigurationKind, r.previousSiteConfig.ID)
		return &id
	}

	return nil
}

func (r SiteConfigurationChangeResolver) Author(ctx context.Context) (*UserResolver, error) {
	if r.siteConfig.AuthorUserID == 0 {
		return nil, nil
	}

	user, err := UserByIDInt32(ctx, r.db, r.siteConfig.AuthorUserID)
	if err != nil {
		return nil, err
	}

	return user, nil
}

// TODO: Implement redaction.
func (r SiteConfigurationChangeResolver) Diff() string {
	// FIXME
	if r.previousSiteConfig == nil {
		return ""
	}

	return cmp.Diff(r.siteConfig.Contents, r.previousSiteConfig.Contents)
}

// FIXME: Doesn't look like its returning the correct value.
// FIX last and this will be fixed
// Part of pagination
func (r SiteConfigurationChangeResolver) CreatedAt() gqlutil.DateTime {
	return gqlutil.DateTime{Time: r.siteConfig.CreatedAt}
}

func (r SiteConfigurationChangeResolver) UpdatedAt() gqlutil.DateTime {
	return gqlutil.DateTime{Time: r.siteConfig.UpdatedAt}
}
