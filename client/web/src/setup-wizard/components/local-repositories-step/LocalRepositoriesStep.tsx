import { ChangeEvent, FC, forwardRef, HTMLAttributes, InputHTMLAttributes, useEffect, useState } from 'react'

import { mdiGit } from '@mdi/js'
import classNames from 'classnames'

import { ErrorLike } from '@sourcegraph/common'
import { useQuery } from '@sourcegraph/http-client'
import { TelemetryProps } from '@sourcegraph/shared/src/telemetry/telemetryService'
import {
    Alert,
    Button,
    Container,
    ErrorAlert,
    H4,
    Icon,
    Input,
    LoaderInput,
    Text,
    useDebounce,
    Tooltip,
} from '@sourcegraph/wildcard'

import { DiscoverLocalRepositoriesResult, DiscoverLocalRepositoriesVariables } from '../../../graphql-operations'
import { CodeHostExternalServiceAlert } from '../CodeHostExternalServiceAlert'
import { ProgressBar } from '../ProgressBar'
import { CustomNextButton, FooterWidget } from '../setup-steps'

import { callFilePicker } from './helpers'
import { useLocalRepositoriesPaths, useLocalRepositories } from './hooks'
import { DISCOVER_LOCAL_REPOSITORIES } from './queries'

import styles from './LocalRepositoriesStep.module.scss'

interface LocalRepositoriesStepProps extends TelemetryProps, HTMLAttributes<HTMLDivElement> {
    description?: boolean
    progressBar?: boolean
}

export const LocalRepositoriesStep: FC<LocalRepositoriesStepProps> = ({
    telemetryService,
    description = true,
    progressBar = true,
    ...attributes
}) => {
    const { paths, autogeneratedPaths, loading, error, setPaths } = useLocalRepositoriesPaths()

    useEffect(() => {
        telemetryService.log('SetupWizardLandedAddLocalCode')
    }, [telemetryService])

    const handleNextButtonClick = (): void => {
        if (!paths) {
            telemetryService.log('SetupWizardSkippedAddLocalCode')
        }
    }

    return (
        <div {...attributes}>
            {description && <Text className="mb-2">Add your local repositories from your disk.</Text>}

            <CodeHostExternalServiceAlert />

            <Container className={styles.content}>
                {!loading && (
                    <LocalRepositoriesForm
                        isFilePickerAvailable={true}
                        error={error}
                        directoryPaths={paths}
                        onDirectoryPathsChange={setPaths}
                    />
                )}

                {!loading && autogeneratedPaths.length > 0 && (
                    <BuiltInRepositories directoryPaths={autogeneratedPaths} />
                )}
            </Container>

            {progressBar && (
                <FooterWidget>
                    <ProgressBar />
                </FooterWidget>
            )}

            <CustomNextButton
                label={paths.length > 0 ? 'Next' : 'Skip'}
                tooltip={paths.length === 0 ? 'You can get back to this step later' : ''}
                onClick={handleNextButtonClick}
            />
        </div>
    )
}

interface LocalRepositoriesFormProps {
    isFilePickerAvailable: boolean
    error: ErrorLike | undefined
    directoryPaths: string[]
    onDirectoryPathsChange: (paths: string[]) => void
}

const LocalRepositoriesForm: FC<LocalRepositoriesFormProps> = props => {
    const { isFilePickerAvailable, error, directoryPaths, onDirectoryPathsChange } = props

    const [internalPaths, setInternalPaths] = useState(directoryPaths)

    const { repositories, loading, loaded } = useLocalRepositories({
        skip: !!error,
        paths: directoryPaths,
    })

    // By default, input is disabled so this callback won't be fired
    // but in case if backend-based file picker isn't supported in OS
    // that is running sg instance we fall back on common input where user
    // should file path manually
    const handleInputChange = (event: ChangeEvent<HTMLInputElement>): void => {
        setInternalPaths(event.target.value.split(','))
    }

    const handlePathReset = (): void => {
        setInternalPaths([])
        onDirectoryPathsChange([])
    }

    const debouncedInternalPaths = useDebounce(internalPaths, 1000)

    // Sync internal state with parent logic
    useEffect(() => {
        onDirectoryPathsChange(debouncedInternalPaths)
    }, [debouncedInternalPaths, onDirectoryPathsChange])

    const handlePathsPickClick = async (): Promise<void> => {
        const paths = await callFilePicker()

        if (paths !== null) {
            onDirectoryPathsChange(paths)
        }
    }

    // Use internal path only if backend-based file picker is unavailable
    const paths = isFilePickerAvailable ? directoryPaths : internalPaths
    const initialState = !error && !loading && paths.length === 0 && repositories.length === 0
    const zeroResultState = loaded && paths.length > 0 && !error && repositories.length === 0

    return (
        <>
            <header>
                <Input
                    as={InputWithActions}
                    value={paths.join(', ')}
                    label="Directory path"
                    isFilePickerMode={isFilePickerAvailable}
                    placeholder="/Users/user-name/Projects/"
                    message="Pick a git directory or folder that contains multiple git folders"
                    isProcessing={loading}
                    className={styles.filePicker}
                    // eslint-disable-next-line @typescript-eslint/no-misused-promises
                    onPickPath={handlePathsPickClick}
                    onPathReset={handlePathReset}
                    onChange={handleInputChange}
                />
            </header>

            {error && <ErrorAlert error={error} className="mt-3" />}

            {!error && (
                <ul className={styles.list}>
                    {repositories.map(repository => (
                        <li key={repository.path} className="d-flex">
                            <Icon svgPath={mdiGit} size="md" aria-hidden={true} className="mt-1 mr-3" />
                            <div className="d-flex flex-column">
                                <Text weight="medium" className="mb-0">
                                    {repository.name}
                                </Text>
                                <Text size="small" className="text-muted mb-0">
                                    {repository.path}
                                </Text>
                            </div>
                        </li>
                    ))}
                </ul>
            )}

            {zeroResultState && (
                <Alert variant="primary" className="mt-3 mb-0">
                    <H4>We couldn't resolve any git repositories by the current path</H4>
                    Try to use different path that contains .git repositories
                </Alert>
            )}

            {initialState && (
                <Alert variant="secondary" className="mt-3 mb-0">
                    <Text className="mb-0 text-muted">
                        Pick a path to see a list of local repositories that you want to have in the Sourcegraph App
                    </Text>
                </Alert>
            )}
        </>
    )
}

interface InputWithActionsProps extends InputHTMLAttributes<HTMLInputElement> {
    isFilePickerMode: boolean
    isProcessing: boolean
    onPickPath: () => void
    onPathReset: () => void
}

/**
 * Renders either file picker input (non-editable but clickable and with "pick a path" action button or
 * simple input where user can input path manually.
 */
const InputWithActions = forwardRef<HTMLInputElement, InputWithActionsProps>(function InputWithActions(props, ref) {
    const { isFilePickerMode, isProcessing, onPickPath, onPathReset, className, value, ...attributes } = props

    return (
        <div className={styles.inputRoot}>
            <Tooltip content={isFilePickerMode ? value : undefined}>
                <LoaderInput
                    loading={isProcessing}
                    className={styles.inputLoader}
                    onClick={isFilePickerMode ? onPickPath : undefined}
                >
                    {/* eslint-disable-next-line react/forbid-elements */}
                    <input
                        {...attributes}
                        ref={ref}
                        value={value}
                        disabled={isFilePickerMode}
                        className={classNames(className, styles.input, { [styles.inputWithAction]: isFilePickerMode })}
                    />
                </LoaderInput>
            </Tooltip>
            {isFilePickerMode && (
                <Button size="sm" type="button" variant="primary" className={styles.pickPath} onClick={onPickPath}>
                    Pick a path
                </Button>
            )}

            <Button size="sm" variant="secondary" className={styles.resetPath} onClick={onPathReset}>
                Reset path
            </Button>
        </div>
    )
})

interface BuiltInRepositoriesProps {
    directoryPaths: string[]
}

const BuiltInRepositories: FC<BuiltInRepositoriesProps> = props => {
    const { directoryPaths } = props

    const { data: repositoriesData, loading } = useQuery<
        DiscoverLocalRepositoriesResult,
        DiscoverLocalRepositoriesVariables
    >(DISCOVER_LOCAL_REPOSITORIES, {
        variables: { paths: directoryPaths },
    })

    const totalNumberOfRepositories = repositoriesData?.localDirectories.repositories.length ?? 0

    if (loading || !repositoriesData || totalNumberOfRepositories === 0) {
        return null
    }

    const foundRepositories = repositoriesData.localDirectories.repositories

    return (
        <section className="mt-4">
            <hr />
            <H4 className="mt-3 mb-1">Built-in repositories</H4>
            <Text size="small" className="text-muted">
                You're running the Sourcegraph app from your terminal. We found the repositories below in your path.
            </Text>
            <ul className={styles.list}>
                {foundRepositories.map(repository => (
                    <li key={repository.path} className={classNames('d-flex')}>
                        <Icon svgPath={mdiGit} size="md" aria-hidden={true} className="mt-1 mr-3" />
                        <div className="d-flex flex-column">
                            <Text weight="medium" className="mb-0">
                                {repository.name}
                            </Text>
                            <Text size="small" className="text-muted mb-0">
                                {repository.path}
                            </Text>
                        </div>
                    </li>
                ))}
            </ul>
        </section>
    )
}