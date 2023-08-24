package managedservicesplatform

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/hashicorp/terraform-cdk-go/cdktf"
	"github.com/sourcegraph/conc/panics"
)

type CDKTF struct {
	app    cdktf.App
	stacks []string

	terraformVersion string
}

// OutputDir is the directory that Synthesize will place output in.
func (c CDKTF) OutputDir() string {
	if s := c.app.Outdir(); s != nil {
		return *s
	}
	return ""
}

// Synthesize all resources to the output directory that was originally
// configured.
func (c CDKTF) Synthesize() error {
	// CDKTF is prone to panics for no good reason, so make a best-effort
	// attempt to capture them.
	var catcher panics.Catcher
	catcher.Try(c.app.Synth)
	if err := catcher.Recovered().AsError(); err != nil {
		return err
	}

	// Generate an asdf tool-version file for convenience to align Terraform
	// with the configured Terraform versions of the generated stacks.
	toolVersionsPath := filepath.Join(c.OutputDir(), ".tool-versions")
	if err := os.WriteFile(toolVersionsPath,
		[]byte(fmt.Sprintf("terraform %s", c.terraformVersion)),
		os.ModePerm); err != nil {
		return err
	}

	return nil
}

func (c CDKTF) Stacks() []string {
	return c.stacks
}