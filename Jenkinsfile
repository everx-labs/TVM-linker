G_giturl = "git@github.com:tonlabs/tvm_linker.git"
G_gitcred = "LaninSSHgit"
G_container = "alanin/container-llvm:latest"
G_promoted_branch = "origin/master"
G_buildstatus = "NotSet"
G_teststatus = "NotSet"
G_dockerimage = "NotSet"
C_PROJECT = "NotSet"
C_COMMITER = "NotSet"
C_HASH = "NotSet"
C_TEXT = "NotSet"


// Deploy chanel
DiscordURL = "https://discordapp.com/api/webhooks/496992026932543489/4exQIw18D4U_4T0H76bS3Voui4SyD7yCQzLP9IRQHKpwGRJK1-IFnyZLyYzDmcBKFTJw"

pipeline {
    parameters {
        booleanParam (
            defaultValue: false,
            description: 'Promote image built to be used as latest',
            name : 'FORCE_PROMOTE_LATEST'
        )
    }

    agent none
    options {
        buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '', numToKeepStr: '10')
        disableConcurrentBuilds()
        parallelsAlwaysFailFast()
    }
    stages {
    	    stage('Build docker-image') {
                    agent {
                        node {label 'master'}
                    }
                    stages {
                        stage('Build') {
                            steps {
                                script {
                                    G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                                     sshagent (credentials: [G_gitcred]) {
                                    withEnv(['DOCKER_BUILDKIT=1']) {
                                        app_image = docker.build(
                                             "${G_dockerimage}",
                                            '--label "git-commit=${GIT_COMMIT}" --ssh default .'
                                         )
                                     }
                                    }
                                }
                            }
                        }
                        stage('Test') { 
                            steps {
                                script {
                                    sh '''
                                        echo "Ok."
                                    '''
                                }
                            }
                        }
                        stage('Push docker-image') {
                            steps {
                                script {
                                    docker.withRegistry('', 'dockerhubLanin') {
                                        app_image.push()
                                    }
                                }
                            }
                            post {
                                success {
                                    script{
                                        G_buildstatus = "success"
                                    }
                                }
                                failure {script{G_buildstatus = "failure"}}
                            }
                        }
                    }
                }
            stage ('Tag as latest') {
            when {
                expression {
                    GIT_BRANCH = 'origin/' + sh(returnStdout: true, script: 'git rev-parse --abbrev-ref HEAD').trim()
                    return GIT_BRANCH == G_promoted_branch || params.FORCE_PROMOTE_LATEST
                } 
            }
            agent {
                node {label 'master'}
            }
            steps {
                script {
                    docker.withRegistry('', 'dockerhubLanin') {
                        docker.image("${G_dockerimage}").push('latest')
                    }
                }
            }        
        }
            }
        
    post {
        always {
            node ('master') {
                script {
                    cleanWs notFailBuild: true
                    currentBuild.description = C_TEXT
                    string DiscordFooter = "Build duration is " + currentBuild.durationString
                    DiscordTitle = "Job ${JOB_NAME} from GitHub " + C_PROJECT
                    DiscordDescription = C_COMMITER + " pushed commit " + C_HASH + " by " + C_AUTHOR + " with a message '" + C_TEXT + "'" + "\n" \
                    + "Build number ${BUILD_NUMBER}" + "\n" \
                    + "Build: **" + G_buildstatus + "**" + "\n" \
                    + "Tests: **" + G_teststatus + "**" + "\n"
                    discordSend description: DiscordDescription, footer: DiscordFooter, link: RUN_DISPLAY_URL, successful: currentBuild.resultIsBetterOrEqualTo('SUCCESS'), title: DiscordTitle, webhookURL: DiscordURL
                }
            }
        }

    }
}
