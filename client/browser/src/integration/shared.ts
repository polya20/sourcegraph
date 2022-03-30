import puppeteer from 'puppeteer'

import { isDefined } from '@sourcegraph/common'
import { percySnapshot as percySnapshotCommon } from '@sourcegraph/shared/src/testing/driver'

/**
 * Find a tab that contains the browser extension's after-install page (url
 * ending in `/after_install.html`) and, if found, close it.
 *
 * The after-install page is opened automatically when the browser extension is
 * installed. In tests, this means that it's opened automatically every time we
 * start the browser (with the browser extension loaded).
 */
export async function closeInstallPageTab(browser: puppeteer.Browser): Promise<void> {
    // Sometimes the install page isn't open by the time we try to close it.
    // Try to close the install page as quickly as possible, retry until it's open.
    let tries = 0
    while (tries < 5) {
        const pages = await browser.pages()
        const installPage = pages.find(page => page.url().endsWith('/after_install.html'))
        if (installPage) {
            await installPage.close()
            return
        }
        await new Promise(resolve => setTimeout(resolve, 200))
        tries++
    }
}

const extractExtensionStyles = async (page: puppeteer.Page): Promise<string> =>
    page.evaluate(async () => {
        const styleSheets = [...document.styleSheets]
            .map(styleSheet => styleSheet.href)
            .filter(isDefined)
            .filter(href => new URL(href).protocol === 'chrome-extension:')

        let styles = ''

        for (const styleSheet of styleSheets) {
            const response = await fetch(styleSheet)
            const css = await response.text()
            styles += css
            styles += '\n'
        }

        return styles
    })

export const percySnapshot: typeof percySnapshotCommon = async (page, name, options) => {
    const extensionStyles = await extractExtensionStyles(page)
    const percyCSS = extensionStyles + (options?.percyCSS || '')

    return percySnapshotCommon(page, name, { ...options, percyCSS })
}
