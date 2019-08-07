G_giturl = "git@github.com:tonlabs/TVM-linker.git"
G_gitcred = "LaninSSHgit"
G_container = "alanin/container-llvm:latest"
G_promoted_branch = "origin/master"
G_dockerimage = "NotSet"
G_buildstatus = "NotSet"
G_clangstatus = "NotSet"
G_llvmstatus = "NotSet"
G_teststatus = "NotSet"
G_Wbuildstatus = "NotSet"
G_Wclangstatus = "NotSet"
G_Wllvmstatus = "NotSet"
G_Wteststatus = "NotSet"
G_workdir = "/opt/work"
G_ramdir = "/media/ramdisk/toolchain"
C_PROJECT = "NotSet"
C_COMMITER = "NotSet"
C_HASH = "NotSet"
C_TEXT = "NotSet"
def getVar(Gvar) {return Gvar}
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
        buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '', numToKeepStr: '20')
        disableConcurrentBuilds()
        lock('RamDrive')
    }
    environment {
        WORKDIR = getVar(G_workdir)
        RAMDIR = getVar(G_ramdir)
    }
    stages {
    	 stage('Build docker-image') {
		agent {
                	node {label 'master'}
            	}
         	stage('Build') {
			steps {
                        	script {
                                    G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                                    app_image = docker.build(
                                        "${G_dockerimage}", 
                                        '--label "git-commit=${GIT_COMMIT}" .'
                                    )
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
