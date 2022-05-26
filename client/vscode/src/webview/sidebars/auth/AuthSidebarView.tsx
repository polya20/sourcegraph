import React, { useCallback, useEffect, useMemo, useState } from 'react'

import { VSCodeButton, VSCodeLink } from '@vscode/webview-ui-toolkit/react'
import classNames from 'classnames'

import { Form } from '@sourcegraph/branded/src/components/Form'
import { LoaderInput } from '@sourcegraph/branded/src/components/LoaderInput'
import { currentAuthStateQuery } from '@sourcegraph/shared/src/auth'
import { CurrentAuthStateResult, CurrentAuthStateVariables } from '@sourcegraph/shared/src/graphql-operations'
import { Alert, Typography, Link } from '@sourcegraph/wildcard'

import {
    VSCE_LINK_DOTCOM,
    VSCE_LINK_MARKETPLACE,
    VSCE_LINK_AUTH,
    VSCE_LINK_TOKEN_CALLBACK,
    VSCE_LINK_TOKEN_CALLBACK_TEST,
    VSCE_LINK_USER_DOCS,
    VSCE_SIDEBAR_PARAMS,
} from '../../../common/links'
import { WebviewPageProps } from '../../platform/context'

import styles from './AuthSidebarView.module.scss'
interface AuthSidebarViewProps
    extends Pick<WebviewPageProps, 'extensionCoreAPI' | 'platformContext' | 'instanceURL' | 'authenticatedUser'> {}

interface AuthSidebarCtaProps extends Pick<WebviewPageProps, 'platformContext'> {}

/**
 * Rendered by sidebar in search-home state when user doesn't have a valid access token.
 */
export const AuthSidebarView: React.FunctionComponent<React.PropsWithChildren<AuthSidebarViewProps>> = ({
    instanceURL,
    extensionCoreAPI,
    platformContext,
    authenticatedUser,
}) => {
    const [state, setState] = useState<'initial' | 'validating' | 'success' | 'failure'>('initial')
    const [hasAccount, setHasAccount] = useState(authenticatedUser?.username !== undefined)
    const [usePrivateInstance, setUsePrivateInstance] = useState(false)
    const [useAccessToken, setUseAccessToken] = useState(false)
    const signUpURL = VSCE_LINK_AUTH('sign-up')
    const instanceHostname = useMemo(() => new URL(instanceURL).hostname, [instanceURL])
    const [hostname, setHostname] = useState(instanceHostname)
    const [accessToken, setAccessToken] = useState<string | undefined>('initial')
    const [endpointUrl, setEndpointUrl] = useState(instanceURL)
    const isSourcegraphDotCom = useMemo(() => {
        const hostname = new URL(instanceURL).hostname
        if (hostname === 'sourcegraph.com' || hostname === 'www.sourcegraph.com') {
            return VSCE_LINK_TOKEN_CALLBACK
        }
        if (hostname === 'sourcegraph.test') {
            return VSCE_LINK_TOKEN_CALLBACK_TEST
        }
        return null
    }, [instanceURL])

    useEffect(() => {
        // Get access token from setting
        if (accessToken === 'initial') {
            extensionCoreAPI.getAccessToken
                .then(token => {
                    setAccessToken(token)
                    // If an access token and endpoint url exist at initial load,
                    // assumes the extension was started with a bad token because
                    // user should be autheticated automatically if token is valid
                    if (endpointUrl && token) {
                        setState('failure')
                    }
                })
                .catch(error => console.log(error))
        }
    }, [accessToken, endpointUrl, extensionCoreAPI.getAccessToken])

    const onTokenInputChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
        setAccessToken(event.target.value)
    }, [])

    const onInstanceURLInputChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
        setEndpointUrl(event.target.value)
    }, [])

    const validateAccessToken: React.FormEventHandler<HTMLFormElement> = (event): void => {
        event.preventDefault()
        if (state !== 'validating' && accessToken) {
            const authStateVariables = {
                request: currentAuthStateQuery,
                variables: {},
                mightContainPrivateInfo: true,
                overrideAccessToken: accessToken,
                overrideSourcegraphURL: '',
            }
            if (usePrivateInstance) {
                setHostname(new URL(endpointUrl).hostname)
                authStateVariables.overrideSourcegraphURL = endpointUrl
            }
            setState('validating')
            const currentAuthStateResult = platformContext
                .requestGraphQL<CurrentAuthStateResult, CurrentAuthStateVariables>(authStateVariables)
                .toPromise()

            currentAuthStateResult
                .then(async ({ data }) => {
                    if (data?.currentUser) {
                        setState('success')
                        // Update access token and instance url in user config for the extension
                        await extensionCoreAPI.setEndpointUri(endpointUrl, accessToken)
                        return
                    }
                    setState('failure')
                    return
                })
                // v2/debt: Disambiguate network vs auth errors like we do in the browser extension.
                .catch(() => setState('failure'))
        }
        // If successful, update setting. This form will no longer be rendered
    }

    const onSignUpClick = (): void => {
        setHasAccount(true)
        platformContext.telemetryService.log('VSCESidebarCreateAccount')
    }

    const onBrowserSigninClick = (): void => {
        setHasAccount(true)
        setUsePrivateInstance(false)
        setUseAccessToken(!useAccessToken)
        platformContext.telemetryService.log('VSCESidebarBrowserSignin')
    }

    const onEnterpriseSigninClick = (): void => {
        setUsePrivateInstance(!usePrivateInstance)
        setUseAccessToken(!useAccessToken)
        platformContext.telemetryService.log('VSCESidebarEnterpriseSignin')
    }

    if (state === 'success') {
        // This form should no longer be rendered as the extension context
        // will be invalidated. We should show a notification that the accessToken
        // has successfully been updated.
        return null
    }

    const renderCommon = (content: JSX.Element): JSX.Element => (
        <>
            <div className={classNames(styles.ctaContainer)}>
                <Form onSubmit={validateAccessToken}>
                    <button type="button" className={classNames('btn btn-outline-secondary', styles.ctaTitle)}>
                        <Typography.H5 className="flex-grow-1">Search your private code</Typography.H5>
                    </button>
                    {content}
                </Form>
            </div>
        </>
    )

    if (!hasAccount && !accessToken) {
        return renderCommon(
            <div>
                <p className={classNames(styles.ctaParagraph)}>
                    Create an account to search across your private repositories and access advanced features: search
                    multiple repositories & commit history, monitor code changes, save searches, and more.
                </p>
                <Link to={signUpURL}>
                    <VSCodeButton
                        type="button"
                        onClick={onSignUpClick}
                        className={classNames(
                            'btn my-1 p-0',
                            styles.ctaButton,
                            styles.ctaButtonWrapperWithContextBelow
                        )}
                        autofocus={false}
                        disabled={usePrivateInstance}
                    >
                        Create an account
                    </VSCodeButton>
                </Link>
                <VSCodeLink className="my-0" onClick={() => setHasAccount(true)}>
                    Have an account?
                </VSCodeLink>
            </div>
        )
    }

    return renderCommon(
        <>
            <p className={classNames(styles.ctaParagraph)}>
                {hasAccount
                    ? 'Connect your Sourcegraph account to enable searches on your private code.'
                    : 'Create an account to search across your private repositories and access advanced features: search multiple repositories & commit history, monitor code changes, save searches, and more.'}
            </p>
            {/* ----------- LOGIN WITH BROWSER FOR CLOUD USERS ONLY ----------- */}
            {isSourcegraphDotCom && (
                <Link to={isSourcegraphDotCom}>
                    <VSCodeButton
                        type="button"
                        onClick={() => onBrowserSigninClick()}
                        className={classNames(
                            'btn my-1 p-0',
                            styles.ctaButton,
                            styles.ctaButtonWrapperWithContextBelow
                        )}
                        autofocus={false}
                        disabled={usePrivateInstance}
                    >
                        Connect in Browser
                    </VSCodeButton>
                </Link>
            )}
            {/* ----------- LOGIN WITH ACCESS TOKEN / AS ENTERPRISE USER ----------- */}
            {useAccessToken && (
                <div className={classNames(styles.tokenLoginContainer)}>
                    <button type="button" className={classNames('btn btn-outline-secondary', styles.ctaTitle)}>
                        <Typography.H5 className="flex-grow-1">
                            {usePrivateInstance ? 'Connect to a private instance' : 'Or login with an Access Token'}
                        </Typography.H5>
                    </button>
                    <p className={classNames(styles.ctaParagraph)}>
                        Sign in by entering an access token created through your Sourcegraph user settings. See our{' '}
                        {/* eslint-disable-next-line react/forbid-elements */}{' '}
                        <a href={VSCE_LINK_USER_DOCS}>user docs</a> for a video guide on how to create an access token.
                    </p>
                    {usePrivateInstance && (
                        <p className={classNames(styles.ctaButtonWrapperWithContextBelow)}>
                            <LoaderInput loading={state === 'validating'}>
                                <Typography.Label htmlFor="instance-url-input">
                                    Sourcegraph Instance URL
                                </Typography.Label>
                                <input
                                    className={classNames('input form-control', styles.ctaInput)}
                                    id="instance-url-input"
                                    type="url"
                                    name="instance-url"
                                    onChange={onInstanceURLInputChange}
                                    required={true}
                                    autoFocus={true}
                                    spellCheck={false}
                                    disabled={state === 'validating'}
                                    placeholder="ex https://sourcegraph.example.com"
                                />
                            </LoaderInput>
                        </p>
                    )}
                    <p className={classNames(styles.ctaButtonWrapperWithContextBelow)}>
                        <LoaderInput loading={state === 'validating'}>
                            <Typography.Label htmlFor="access-token-input">Access Token</Typography.Label>
                            <input
                                className={classNames('input form-control', styles.ctaInput)}
                                id="access-token-input"
                                value={accessToken}
                                type="text"
                                onChange={onTokenInputChange}
                                name="token"
                                required={true}
                                autoFocus={!usePrivateInstance}
                                spellCheck={false}
                                disabled={state === 'validating'}
                                placeholder="ex 6dfc880b320dff712d9f6cfcac5cbd13ebfad1d8"
                            />
                        </LoaderInput>
                    </p>
                    <VSCodeButton
                        type="submit"
                        disabled={state === 'validating'}
                        className={classNames(
                            'btn my-1 p-0',
                            styles.ctaButton,
                            styles.ctaButtonWrapperWithContextBelow
                        )}
                    >
                        Authenticate account
                    </VSCodeButton>
                    {/* ----------- ERROR DISPLAY ----------- */}
                    {state === 'failure' && (
                        <Alert variant="danger" className={classNames(styles.ctaParagraph, 'my-1')}>
                            Unable to verify your access token for {hostname}. Please try again with a new access token
                            or restart VS Code if the instance URL has been updated.
                        </Alert>
                    )}
                </div>
            )}
            <p className="my-0">
                <VSCodeLink onClick={() => onEnterpriseSigninClick()}>
                    {!usePrivateInstance ? 'Need to connect to a private instance?' : 'Not a private instance user?'}
                </VSCodeLink>
            </p>
            <p className="my-0">
                <VSCodeLink href={signUpURL} onClick={onSignUpClick}>
                    Create an account
                </VSCodeLink>
            </p>
        </>
    )
}

export const AuthSidebarCta: React.FunctionComponent<React.PropsWithChildren<AuthSidebarCtaProps>> = ({
    platformContext,
}) => {
    const onLinkClick = (type: 'Sourcegraph' | 'Extension'): void =>
        platformContext.telemetryService.log(`VSCESidebarLearn${type}Click`)

    return (
        <div>
            <button type="button" className={classNames('btn btn-outline-secondary', styles.ctaTitle)}>
                <Typography.H5 className="flex-grow-1">Welcome</Typography.H5>
            </button>
            <p className={classNames(styles.ctaParagraph)}>
                The Sourcegraph extension allows you to search millions of open source repositories without cloning them
                to your local machine.
            </p>
            <p className={classNames(styles.ctaParagraph)}>
                Developers use Sourcegraph every day to onboard to new code bases, find code to reuse, resolve
                incidents, fix security vulnerabilities, and more.
            </p>
            <div className={classNames(styles.ctaParagraph)}>
                <p className="mb-0">Learn more:</p>
                <VSCodeLink href={VSCE_LINK_DOTCOM + VSCE_SIDEBAR_PARAMS} onClick={() => onLinkClick('Sourcegraph')}>
                    Sourcegraph.com
                </VSCodeLink>
                <br />
                <VSCodeLink href={VSCE_LINK_MARKETPLACE} onClick={() => onLinkClick('Extension')}>
                    Sourcegraph VS Code extension
                </VSCodeLink>
            </div>
        </div>
    )
}
