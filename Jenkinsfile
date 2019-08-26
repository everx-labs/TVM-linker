@Library('infrastructure-jenkins-shared-library@master')_

G_gitcred = "LaninSSHgit"
G_promoted_branch = 'origin/master'
G_buildstatus = 'NotSet'
G_teststatus = 'NotSet'

pipeline {
    parameters {
        booleanParam (
            defaultValue: false,
            description: 'Promote image built to be used as latest',
            name : 'FORCE_PROMOTE_LATEST'
        )
    }
    agent {
        node {label 'master'}
    }
    options {
        buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '', numToKeepStr: '10')
        disableConcurrentBuilds()
        parallelsAlwaysFailFast()
    }
    stages {
        stage ('Build') {
            steps {
                script {
                    G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                    sshagent (credentials: [G_gitcred]) {
                        withEnv(["DOCKER_BUILDKIT=1", "BUILD_INFO=${env.BUILD_TAG}:${GIT_COMMIT}"]) {
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
                    G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                    docker.image(G_dockerimage).inside("-u root") {
                        sh 'tvm_linker --version'
                    }
                }
            }
        }
        stage ('Test') {
            agent {
                docker {
                    image "${G_dockerimage}"
                    alwaysPull false
                    args '-u root'
                }
            }
            steps {
                script {
                    sh 'apk add libgcc gcompat libc6-compat python'
                    sh '/usr/bin/tvm_linker --version'
                    sh 'cd tvm_linker && python test_suite.py --linker-path=/usr/bin/tvm_linker'
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
                failure {script{G_buildstatus = "failure"}}
            }
        }
        stage('Test in compiler-kit') {
            steps {
                script {
                    G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                    def params = [
                      [$class: 'StringParameterValue', name: 'dockerimage_tvm_linker', value: "${G_dockerimage}"]
                    ]
                    build job : "Infrastructure/compilers/master", parameters : params
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
        stage ('Tag as latest') {
            when {
                expression {
                    // GIT_BRANCH = 'origin/' + sh(returnStdout: true, script: 'git rev-parse --abbrev-ref HEAD').trim()
                    GIT_BRANCH = "origin/${BRANCH_NAME}"
                    return GIT_BRANCH == G_promoted_branch || params.FORCE_PROMOTE_LATEST
                }
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
            notifyTeam(
                buildstatus: G_buildstatus
            ) 
        }
    }
}
