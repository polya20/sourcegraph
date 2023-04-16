import React from 'react'

import classNames from 'classnames'

import { ChatMessage } from '@sourcegraph/cody-shared/src/chat/transcript/messages'

import { CodySvg } from '../utils/icons'

import { BlinkingCursor } from './BlinkingCursor'
import { CodeBlocks } from './CodeBlocks'
import { ContextFiles, FileLinkProps } from './ContextFiles'

import styles from './TranscriptItem.module.css'

/**
 * CSS class names used for the {@link TranscriptItem} component.
 */
export interface TranscriptItemClassNames {
    transcriptItemClassName?: string
    humanTranscriptItemClassName?: string
    transcriptItemParticipantClassName?: string
    codeBlocksCopyButtonClassName?: string
}

/**
 * A single message in the chat trans cript.
 */
export const TranscriptItem: React.FunctionComponent<
    {
        message: ChatMessage
        inProgress: boolean
        fileLinkComponent: React.FunctionComponent<FileLinkProps>
    } & TranscriptItemClassNames
> = ({
    message,
    inProgress,
    fileLinkComponent,
    transcriptItemClassName,
    humanTranscriptItemClassName,
    transcriptItemParticipantClassName,
    codeBlocksCopyButtonClassName,
}) => (
    <div
        className={classNames(
            styles.row,
            transcriptItemClassName,
            message.speaker === 'human' ? humanTranscriptItemClassName : null
        )}
    >
        <header className={classNames(styles.participant, transcriptItemParticipantClassName)}>
            <h2 className={styles.participantName}>
                {message.speaker === 'assistant' ? (
                    <>
                        <CodySvg className={styles.participantAvatar} /> Cody
                    </>
                ) : (
                    'Me'
                )}
            </h2>
        </header>
        <div className={classNames(styles.content)}>
            {message.displayText ? (
                <>
                    <CodeBlocks displayText={message.displayText} copyButtonClassName={codeBlocksCopyButtonClassName} />
                    {message.contextFiles && message.contextFiles.length > 0 && (
                        <ContextFiles
                            contextFiles={message.contextFiles}
                            fileLinkComponent={fileLinkComponent}
                            className={styles.contextFiles}
                        />
                    )}
                </>
            ) : inProgress ? (
                <BlinkingCursor />
            ) : null}
        </div>
    </div>
)
